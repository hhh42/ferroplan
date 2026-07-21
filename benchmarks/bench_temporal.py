#!/usr/bin/env python3
"""Temporal benchmark harness: run ferroplan on a domain's instances and validate
each plan with VAL (the IPC plan validator).

Usage: bench_temporal.py <domain_dir> [max_instances] [timeout_s]
  <domain_dir> holds domain.pddl + instances/*.pddl

Emits one JSON object to stdout summarising the domain. Paths to the ferroplan
`ff` binary and VAL `Validate` are taken from $FF and $VAL (with sensible
defaults), so the harness is reproducible and CI/oracle-optional.

VAL and the IPC benchmark instances are NOT vendored in this repo (licences /
size); see COMPARING.md. This script just drives them when present.
"""
import json
import os
import subprocess
import sys
import time

FF = os.environ.get("FF", "ferroplan/target/release/ff")
VAL = os.environ.get("VAL", "VAL/out/bin/Validate")
EPS = "0.001"  # IPC ε-separation tolerance for VAL


def instances(domain_dir):
    idir = os.path.join(domain_dir, "instances")
    fs = [f for f in os.listdir(idir) if f.endswith(".pddl")]
    # natural-ish sort: instance-2 before instance-10
    def key(f):
        digits = "".join(c if c.isdigit() else " " for c in f).split()
        return [int(d) for d in digits] or [0]
    return [os.path.join(idir, f) for f in sorted(fs, key=key)]


def run_one(domain, inst, timeout):
    """Return (status, val, ms, detail). status in solved/unsolved/parse_error/error."""
    t = time.perf_counter()
    try:
        r = subprocess.run(
            [FF, "-o", domain, "-f", inst, "--mode", "temporal"],
            capture_output=True, text=True, timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return "unsolved", None, timeout * 1000, "timeout"
    ms = (time.perf_counter() - t) * 1000
    out, err = r.stdout, r.stderr
    if "parse error" in err.lower() or "parse error" in out.lower():
        line = (err + out).strip().splitlines()
        return "parse_error", None, ms, (line[0] if line else "parse error")[:120]
    plan_lines = [ln for ln in out.splitlines() if ln[:1].isdigit() and ln.rstrip().endswith("]")]
    if not plan_lines:
        return "unsolved", None, ms, "no plan"
    # validate with VAL
    planf = "/tmp/_ferro_%d.plan" % os.getpid()
    with open(planf, "w") as fh:
        fh.write("\n".join(plan_lines) + "\n")
    try:
        v = subprocess.run([VAL, "-t", EPS, domain, inst, planf],
                           capture_output=True, text=True, timeout=60).stdout
        valid = ("Plan valid" in v) or ("Successful plans" in v)
        detail = "" if valid else _val_reason(v)
    except Exception as e:  # VAL missing / crash
        valid = None
        detail = "VAL unavailable: %s" % e
    finally:
        try:
            os.remove(planf)
        except OSError:
            pass
    return "solved", valid, ms, detail


def _val_reason(v):
    for ln in v.splitlines():
        if "unsatisfied" in ln.lower() or "failed because" in ln.lower():
            return ln.strip()[:140]
    return "invalid (no specific reason parsed)"


def main():
    domain_dir = sys.argv[1]
    max_inst = int(sys.argv[2]) if len(sys.argv) > 2 else 20
    timeout = float(sys.argv[3]) if len(sys.argv) > 3 else 10.0
    domain = os.path.join(domain_dir, "domain.pddl")
    insts = instances(domain_dir)[:max_inst]

    def domain_for(inst):
        """Single shared domain.pddl, or the per-instance domains/ layout some
        IPC variants use (parc-printer-2011's domains/domain-N.pddl) — the
        shared-file assumption used to FAIL EVERY instance of those variants
        silently, reading as an honest 0/N."""
        if os.path.exists(domain):
            return domain
        digits = "".join(c if c.isdigit() else " " for c in os.path.basename(inst)).split()
        cand = os.path.join(domain_dir, "domains", "domain-%s.pddl" % (digits[0] if digits else ""))
        return cand if os.path.exists(cand) else domain

    res = {"domain": domain_dir, "total": len(insts), "solved": 0, "valid": 0,
           "invalid": 0, "unsolved": 0, "parse_error": 0, "val_unavailable": 0,
           "first_invalid": "", "first_parse_error": "", "per_instance": []}
    for inst in insts:
        status, valid, ms, detail = run_one(domain_for(inst), inst, timeout)
        name = os.path.basename(inst)
        res["per_instance"].append(
            {"inst": name, "status": status, "valid": valid, "ms": round(ms)})
        if status == "solved":
            res["solved"] += 1
            if valid is True:
                res["valid"] += 1
            elif valid is False:
                res["invalid"] += 1
                if not res["first_invalid"]:
                    res["first_invalid"] = "%s: %s" % (name, detail)
            else:
                res["val_unavailable"] += 1
        elif status == "parse_error":
            res["parse_error"] += 1
            if not res["first_parse_error"]:
                res["first_parse_error"] = "%s: %s" % (name, detail)
        else:
            res["unsolved"] += 1
    print(json.dumps(res))


if __name__ == "__main__":
    main()
