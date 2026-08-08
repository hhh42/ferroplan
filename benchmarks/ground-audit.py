#!/usr/bin/env python3
"""Phase 7 grounding-scale audit: post-restriction typed products, refereed.

The 0.22 Phase 7 threshold routers are denominated in the PER-ACTION
POST-RESTRICTION TYPED PRODUCT — each parameter's typed object count,
narrowed by static unary precondition literals against init, multiplied
out (the exact space `ground::for_each_binding` walks). Two thresholds
read it:

  FF_MCV_THRESHOLD      (1e6)  arms the MCV join order per action
                               (byte-identical by construction — no audit
                               stake, reported for context only);
  FF_FIXPOINT_THRESHOLD (1e13)  routes a PLAIN solve entry through the
                               reached-restricted fixpoint enumeration,
                               which SHIFTS fact-id first-reference order.

The route must therefore be vacuous on everything the boards already
solve — "order changed" is only harmless where nothing grounds today.
This audit asserts exactly that, from the promoted 0.21 raws:

  1. every currently-SOLVED instance on the classical/numeric boards
     (the boards whose solve path is the plain `ground()` entry) sits
     BELOW the fixpoint threshold;
  2. the named Phase 7 gate instances sit ABOVE the bar that unblocks
     them: 2048 i8 above the MCV bar (its wall is static pos-at joins;
     max per-action product 2.1e7), organic-synthesis i01/i11 above the
     fixpoint route bar. caldera i4 (6.4e10) sits BELOW the bar since it
     rose to 1e13: this audit's own first sweep found currently-SOLVED
     products up to 1.62e12 (data-network PROCESS), so no product bar
     admits caldera without re-routing solved domains — caldera's pot
     forfeits to a smarter gate (0.23), on the record (all-dynamic
     preconditions: 1.2e15 / 1.2e25 / 6.4e10);
  3. agricola i1 — 246,879 ops on the plain path, the named
     near-threshold case at 6.3e7 — stays BELOW the route bar (the
     negative control's number is printed, not hidden).

Temporal boards ground through the stratified snap entry, which the
route never touches; their products are reported for MCV context only
(sokoban-t's push is the receipt) and carry no assertion.

Usage:
  python3 benchmarks/ground-audit.py [--threshold 1e13] [--only SUBSTR]
                                     [--raws DIR] [--full]

Default scope: every solved row in --raws (benchmarks/air21) plus the
named gate instances. --full also walks unsolved rows (slow, org-synth
included); --only filters variants by substring.
"""

import glob
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def arg(name, default):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


THRESHOLD = float(arg("--threshold", "1e13"))
ONLY = arg("--only", None)
RAWS = arg("--raws", os.path.join(ROOT, "benchmarks", "air21"))
FULL = "--full" in sys.argv
CORPUS = os.environ.get("FERROPLAN_IPC_CORPUS") or os.path.join(
    ROOT, "benchmarks", ".ipc-corpus")

# ---------------------------------------------------------------- sexp ----


def tokenize(text):
    text = re.sub(r";[^\n]*", "", text)
    return re.findall(r"\(|\)|[^\s()]+", text)


def parse_sexp(tokens, i=0):
    if tokens[i] != "(":
        return tokens[i].upper(), i + 1
    i += 1
    out = []
    while tokens[i] != ")":
        node, i = parse_sexp(tokens, i)
        out.append(node)
    return out, i + 1


def read_sexp(path):
    with open(path, "r", errors="replace") as f:
        node, _ = parse_sexp(tokenize(f.read()))
    return node


def typed_list(items):
    """`a b - t c - u d` -> [(a,t),(b,t),(d,OBJECT)] (case-normalized)."""
    out, pending = [], []
    i = 0
    while i < len(items):
        tok = items[i]
        if tok == "-":
            ty = items[i + 1]
            # `(either a b)` types: fall back to OBJECT (over-approximates
            # the domain, which only ever raises a product — audit-safe).
            if isinstance(ty, list):
                ty = "OBJECT"
            out.extend((v, ty) for v in pending)
            pending = []
            i += 2
        else:
            pending.append(tok)
            i += 1
    out.extend((v, "OBJECT") for v in pending)
    return out


# ------------------------------------------------------------- domain ----


def walk_effect_adds(node, adds):
    if not isinstance(node, list) or not node:
        return
    head = node[0]
    if head == "AND":
        for child in node[1:]:
            walk_effect_adds(child, adds)
    elif head in ("WHEN",):
        walk_effect_adds(node[2], adds)
    elif head == "FORALL":
        walk_effect_adds(node[2], adds)
    elif head in ("AT", "OVER") and len(node) == 3:
        walk_effect_adds(node[2], adds)  # (at start (...)) / (at end (...))
    elif head in ("NOT", "INCREASE", "DECREASE", "ASSIGN",
                  "SCALE-UP", "SCALE-DOWN", "="):
        pass
    else:
        adds.add(head)


def cond_top_atoms(node, out):
    """Top-level positive conjunct atoms of a precondition/condition,
    descending through `and` and the durative `at start/end`/`over all`
    wrappers — the audit analog of ground.rs `static_top_atoms`."""
    if not isinstance(node, list) or not node:
        return
    head = node[0]
    if head == "AND":
        for child in node[1:]:
            cond_top_atoms(child, out)
    elif head in ("AT", "OVER") and len(node) == 3:
        cond_top_atoms(node[2], out)
    elif head in ("NOT", "OR", "IMPLY", "FORALL", "EXISTS", "WHEN",
                  "<", "<=", ">", ">=", "=", "PREFERENCE"):
        pass
    else:
        out.append(node)


class DomainModel:
    def __init__(self, sexp):
        self.type_parent = {}
        self.constants = []
        self.actions = []  # (name, params, cond_atoms)
        self.adds = set()
        for sec in sexp:
            if not isinstance(sec, list) or not sec:
                continue
            head = sec[0]
            if head == ":TYPES":
                for name, parent in typed_list(sec[1:]):
                    self.type_parent[name] = parent
            elif head == ":CONSTANTS":
                self.constants = typed_list(sec[1:])
            elif head in (":ACTION", ":DURATIVE-ACTION"):
                name = sec[1]
                params, cond = [], []
                for i in range(2, len(sec) - 1):
                    if sec[i] == ":PARAMETERS":
                        params = typed_list(sec[i + 1])
                    elif sec[i] in (":PRECONDITION", ":CONDITION"):
                        cond_top_atoms(sec[i + 1], cond)
                    elif sec[i] == ":EFFECT":
                        walk_effect_adds(sec[i + 1], self.adds)
                self.actions.append((name, params, cond))


def is_subtype(ty, ancestor, tp):
    if ancestor == "OBJECT":
        return True
    hops = 0
    while True:
        if ty == ancestor:
            return True
        ty = tp.get(ty)
        if ty is None or hops > len(tp) + 1:
            return False
        hops += 1


def max_product(dom_path, prb_path):
    """(max per-action post-restriction typed product, argmax action)."""
    dom = DomainModel(read_sexp(dom_path))
    objects, init_atoms = [], []
    for sec in read_sexp(prb_path):
        if not isinstance(sec, list) or not sec:
            continue
        if sec[0] == ":OBJECTS":
            objects = typed_list(sec[1:])
        elif sec[0] == ":INIT":
            for a in sec[1:]:
                if isinstance(a, list) and a and a[0] not in ("=", "AT"):
                    init_atoms.append(a)
    all_objects = dom.constants + objects
    tp = dict(dom.type_parent)
    for _, ty in all_objects:
        tp.setdefault(ty, "OBJECT")
    of_type = {}
    for tn in set(tp) | {"OBJECT"}:
        of_type[tn] = [o for o, oty in all_objects if is_subtype(oty, tn, tp)]
    init_unary = {}
    for a in init_atoms:
        if len(a) == 2:
            init_unary.setdefault(a[0], set()).add(a[1])

    best, best_name = 0.0, "-"
    for name, params, cond in dom.actions:
        doms = {v: set(of_type.get(ty, [])) for v, ty in params}
        for atom in cond:
            pred, args = atom[0], atom[1:]
            # static unary restriction, exactly ground.rs's shape: the
            # predicate is never added and the single argument is a param
            if pred in dom.adds or len(args) != 1:
                continue
            v = args[0]
            if v in doms:
                doms[v] &= init_unary.get(pred, set())
        product = 1.0
        for v, _ in params:
            product *= len(doms[v])
        if product > best:
            best, best_name = product, name
    return best, best_name


# -------------------------------------------------------------- boards ----

# air21 board -> (corpus dirs, variant regex, plain-ground()-entry?)
BOARDS = {
    "ipc67-results": (("ipc-2008", "ipc-2011"), r"sequential-satisficing", True),
    "ipc-opt-2008-11": (("ipc-2008", "ipc-2011"), r"sequential-optimal", True),
    "ipc2014-sat": (("ipc-2014",), r"sequential-satisficing", True),
    "ipc2014-agile": (("ipc-2014",), r"sequential-agile", True),
    "ipc2014-opt": (("ipc-2014",), r"sequential-optimal", True),
    "ipc2018-sat": (("ipc-2018",), r"sequential-satisficing", True),
    "ipc2023-agile": (("ipc-2023",), r"-agile$", True),
    "ipc2023-agile-300s": (("ipc-2023",), r"-agile$", True),
    "ipc2023-numeric": (("ipc-2023n",), r"numeric-satisficing", True),
    "ipc2026-numeric": (("ipc-2026n",), r"numeric-2026", True),
    "ipc2026-opt": (("ipc-2026n",), r"-opt-numeric-2026", True),
    # Temporal boards ground through the stratified snap entry — never
    # routed; reported for MCV context, no assertion.
    "ipc67-temporal": (("ipc-2008", "ipc-2011"), r"temporal-satisficing", False),
    "ipc2014-tempo": (("ipc-2014",), r"temporal-satisficing", False),
}

# The pre-registered Phase 7 gate instances: (ipc dir, variant, instance,
# side, bar). Each gate is priced against the threshold that unblocks it:
# 2048's wall is STATIC pos-at joins — the MCV bar (1e6) arms there and is
# byte-identical by construction, no fixpoint route needed (max per-action
# product 2.1e7); org-synth and caldera carry all-dynamic preconditions —
# only the fixpoint route (1e13) gives their joins something to hold.
# agricola is the near-threshold negative control: 6.3e7 sits under the
# route bar (plain path, order untouched) and the number stays printed.
MCV_BAR = 1e6
NAMED = [
    ("ipc-2026n", "2048-numeric-2026", 8, "above", MCV_BAR),
    ("ipc-2018", "organic-synthesis-sequential-satisficing", 1, "above", None),
    ("ipc-2018", "organic-synthesis-sequential-satisficing", 11, "above", None),
    # caldera BELOW the risen bar — the 0.22 recorded negative; a future
    # selectivity-aware gate re-referees it (see the docstring).
    ("ipc-2018", "caldera-sequential-satisficing", 4, "below", None),
    ("ipc-2018", "agricola-sequential-satisficing", 1, "below", None),
]


def instance_files(vdir):
    """label -> (domain path, instance path), mirroring ipc67.py."""
    idir = os.path.join(vdir, "instances")
    shared = os.path.join(vdir, "domain.pddl")
    out = {}
    if not os.path.isdir(idir):
        return out
    for f in os.listdir(idir):
        gs = re.findall(r"\d+", f)
        if not gs:
            continue
        label = int(gs[0]) if len(gs) == 1 else "_".join(gs)
        d = shared if os.path.isfile(shared) else os.path.join(
            vdir, "domains", f"domain-{gs[0]}.pddl")
        if os.path.isfile(d):
            out[str(label)] = (d, os.path.join(idir, f))
    return out


def main():
    if not os.path.isdir(CORPUS):
        sys.exit(f"corpus not found at {CORPUS}")
    solved = {}  # (ipc, variant) -> set of solved instance labels
    unsolved = {}
    for board, (ipcs, pat, asserted) in BOARDS.items():
        raw = os.path.join(RAWS, f"{board}.jsonl")
        if not os.path.isfile(raw):
            print(f"WARN: no raws for {board} at {raw}", file=sys.stderr)
            continue
        with open(raw) as f:
            for line in f:
                r = json.loads(line)
                key = (r["ipc"], r["variant"])
                (solved if r.get("solved") else unsolved).setdefault(
                    key, set()).add(str(r["instance"]))

    failures = []
    print(f"threshold: {THRESHOLD:.0e}  (products are per-action maxima, "
          f"post-restriction)")
    print(f"{'board':22} {'variant':46} {'inst':>6} {'product':>10} verdict")

    seen = set()
    for board, (ipcs, pat, asserted) in BOARDS.items():
        rx = re.compile(pat)
        for ipc in ipcs:
            droot = os.path.join(CORPUS, ipc, "domains")
            if not os.path.isdir(droot):
                continue
            for v in sorted(os.listdir(droot)):
                if not rx.search(v):
                    continue
                if ONLY and ONLY not in v:
                    continue
                key = (ipc, v)
                if (board, key) in seen:
                    continue
                seen.add((board, key))
                files = instance_files(os.path.join(droot, v))
                labels = set(solved.get(key, set()))
                if FULL:
                    labels |= unsolved.get(key, set())
                worst = (0.0, None, "-")
                for label in sorted(labels, key=str):
                    if label not in files:
                        continue
                    d, p = files[label]
                    try:
                        prod, act = max_product(d, p)
                    except Exception as e:  # parse trouble is a finding
                        failures.append(f"{board}/{v}#{label}: parse: {e}")
                        continue
                    is_solved = label in solved.get(key, set())
                    if asserted and is_solved and prod > THRESHOLD:
                        failures.append(
                            f"{board}/{v}#{label}: SOLVED but product "
                            f"{prod:.3e} > {THRESHOLD:.0e} — the route is "
                            f"not vacuous here")
                    if prod > worst[0]:
                        worst = (prod, label, act)
                if worst[1] is not None:
                    tag = "assert" if asserted else "report"
                    flag = "OVER" if worst[0] > THRESHOLD else "ok"
                    print(f"{board:22} {v:46} {worst[1]:>6} "
                          f"{worst[0]:>10.3e} {flag} ({tag}, max of "
                          f"{len(labels)} rows, action {worst[2]})")

    print("\nnamed gate instances:")
    for ipc, variant, inst, side, bar in NAMED:
        bar = bar if bar is not None else THRESHOLD
        vdir = os.path.join(CORPUS, ipc, "domains", variant)
        files = instance_files(vdir)
        if str(inst) not in files:
            failures.append(f"named {variant}#{inst}: instance not found")
            continue
        d, p = files[str(inst)]
        prod, act = max_product(d, p)
        ok = prod > bar if side == "above" else prod <= bar
        print(f"  {variant:52} i{inst:<3} {prod:>10.3e}  want {side} "
              f"{bar:.0e} {'OK' if ok else 'FAIL'} (action {act})")
        if not ok:
            failures.append(
                f"named {variant}#{inst}: product {prod:.3e} not {side} "
                f"{bar:.0e}")

    if failures:
        print("\nAUDIT FAILURES:")
        for f in failures:
            print(f"  {f}")
        sys.exit(1)
    print("\naudit clean: every asserted solved row sits below the "
          "threshold; the named gates sit where Phase 7 priced them.")


if __name__ == "__main__":
    main()
