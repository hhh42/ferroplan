#!/usr/bin/env python3
"""Which domains can VAL validate AT ALL?

Runs VAL against each domain's first instance with an EMPTY plan. An empty
plan is never "valid" (the goal is unmet), but the DISTINCTION we want is
upstream of that: if VAL cannot ingest the domain it says so before judging
anything, and every val=False row on that domain is a harness misattribution
rather than a rejected plan.

Cheap by construction: a VAL that cannot read a domain fails immediately.
"""
import os, re, subprocess, sys, json

C = "benchmarks/.ipc-corpus"
VAL = "benchmarks/.val/VAL/build/bin/Validate"
CORPORA = ["ipc-2008", "ipc-2011", "ipc-2014", "ipc-2018", "ipc-2023",
           "ipc-2023n", "ipc-2026n"]
EMPTY = "/private/tmp/claude-501/-Users-harold-ferroplan/838a1be1-8874-4e21-bbd6-1dd5f60e5bee/scratchpad/empty.plan"
open(EMPTY, "w").close()

# VAL's ways of saying "I cannot ingest this domain/problem", none of which
# are a verdict on a plan.
UNAVAILABLE = ("Problem in domain definition!", "Parser failed",
               "Problem in problem definition!", "Syntax error")

out = {}
for ipc in CORPORA:
    droot = os.path.join(C, ipc, "domains")
    if not os.path.isdir(droot):
        continue
    for v in sorted(os.listdir(droot)):
        vdir = os.path.join(droot, v)
        idir = os.path.join(vdir, "instances")
        dom = os.path.join(vdir, "domain.pddl")
        if not os.path.isdir(idir):
            continue
        names = [f for f in os.listdir(idir) if re.search(r"\d+", f)]
        if not names:
            continue
        names.sort(key=lambda n: int(re.search(r"\d+", n).group()))
        prob = os.path.join(idir, names[0])
        if not os.path.isfile(dom):
            n = int(re.search(r"\d+", names[0]).group())
            dom = os.path.join(vdir, "domains", f"domain-{n}.pddl")
            if not os.path.isfile(dom):
                continue
        try:
            r = subprocess.run([VAL, dom, prob, EMPTY], capture_output=True,
                               text=True, timeout=60)
            blob = (r.stdout or "") + (r.stderr or "")
        except subprocess.TimeoutExpired:
            blob = "<VAL TIMEOUT>"
        hit = next((u for u in UNAVAILABLE if u in blob), None)
        if blob == "<VAL TIMEOUT>":
            hit = "VAL TIMEOUT"
        out[f"{ipc}/{v}"] = hit          # None => VAL can ingest this domain
        if hit:
            print(f"  UNAVAILABLE  {ipc}/{v:<52} {hit}", flush=True)

json.dump(out, open("/private/tmp/claude-501/-Users-harold-ferroplan/"
                    "838a1be1-8874-4e21-bbd6-1dd5f60e5bee/scratchpad/"
                    "val_availability.json", "w"), indent=1)
bad = sum(1 for v in out.values() if v)
print(f"\n{bad} of {len(out)} domains cannot be validated by VAL at all")
