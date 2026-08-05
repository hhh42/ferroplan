"""Semantic differential comparison projected from MFW df511b2."""
from __future__ import annotations
from collections import Counter
import hashlib,json
from .oracle import OracleError,_goal_holds,_states,_transitions,solve
def _canonical(v):return json.dumps(v,sort_keys=True,separators=(",",":")).encode()
def _digest(v):return "sha256:"+hashlib.sha256(_canonical(v)).hexdigest()
def _candidate_result(c):return c["result"] if isinstance(c.get("result"),dict) else c
def _validate_steps(request,candidate):
 p=request["problem"];states=_states(p);edges=_transitions(p,states);index={}
 for e in edges:index.setdefault((e["from"],e["action"],e["to"]),[]).append(e)
 initial=set(p.get("initial_states",[]))
 if len(initial)!=1:return False,{},["STEP_VALIDATION_REQUIRES_SINGLE_INITIAL"]
 cursor=next(iter(initial));totals={"steps":0,"cost":0,"duration":0};errors=[]
 for n,step in enumerate(candidate.get("steps",[])):
  source,target,action=step.get("from"),step.get("to"),step.get("action")
  if source!=cursor:errors.append(f"STEP_{n}_SOURCE_MISMATCH");break
  choices=index.get((source,action,target),[])
  if not choices:errors.append(f"STEP_{n}_TRANSITION_ABSENT");break
  e=choices[0];cursor=target;totals["steps"]+=1;totals["cost"]+=e["cost"];totals["duration"]+=e["duration"]
 valid=not errors and cursor in states and _goal_holds(p.get("goal",{}),states[cursor])
 if not valid and not errors:errors.append("TERMINAL_GOAL_NOT_REACHED")
 return valid,totals,errors
def _objective(kind,t):
 if kind=="classical":return "steps",t["steps"]
 if kind in {"cost_optimal","numeric","preferences","flow_constrained","multi_agent","rdf_derived"}:return "cost",t["cost"]
 if kind=="temporal":return "duration",t["duration"]
 return None
def _signature(r):
 return {"actions":[s.get("action") for s in r.get("steps",[])],"policy":sorted((e.get("state"),e.get("action"),tuple(sorted((o.get("state"),o.get("probability_ppm"),o.get("observation")) for o in e.get("outcomes",[])))) for e in r.get("policy",[])),"decomposition":list(r.get("decomposition",[])),"agents":[s.get("agent") for s in r.get("steps",[])],"tools":[s.get("tool") for s in r.get("steps",[])]}
def _transition_requirements(p):return {(e.get("from"),e.get("action"),e.get("to")):set(e.get("requires",[])) for e in p.get("transitions",[])}
def _task_requirements(p):return {t.get("primitive_action"):set(t.get("requires",[])) for t in p.get("tasks",[]) if t.get("primitive_action") is not None}
def _validate_agent_bindings(problem,candidate,requirements):
 agents={a.get("id"):a for a in problem.get("agents",[])};assigned=Counter();findings=[]
 for n,step in enumerate(candidate.get("steps",[])):
  aid=step.get("agent");agent=agents.get(aid)
  if agent is None:findings.append(f"STEP_{n}_AGENT_ABSENT");continue
  required=requirements(step)
  if not required.issubset(set(agent.get("capabilities",[]))):findings.append(f"STEP_{n}_AGENT_CAPABILITY_UNCOVERED")
  assigned[aid]+=1
 for aid,count in assigned.items():
  a=agents[aid];available=int(a.get("capacity",1))-int(a.get("current_wip",0))
  if count>max(available,0):findings.append(f"AGENT_CAPACITY_EXCEEDED:{aid}")
 return findings
def _validate_multi(request,candidate):
 p=request["problem"];req=_transition_requirements(p);return _validate_agent_bindings(p,candidate,lambda s:req.get((s.get("from"),s.get("action"),s.get("to")),set()))
def _validate_a2a(request,candidate):
 p=request["problem"];req=_task_requirements(p);return _validate_agent_bindings(p,candidate,lambda s:req.get(s.get("action"),set()))
def _validate_mcp(request,candidate):
 p=request["problem"];tools={t.get("id"):t for t in p.get("tools",[])};req=_task_requirements(p);findings=[]
 for n,step in enumerate(candidate.get("steps",[])):
  tool=tools.get(step.get("tool"))
  if tool is None:findings.append(f"STEP_{n}_TOOL_ABSENT");continue
  if not req.get(step.get("action"),set()).issubset(set(tool.get("capabilities",[]))):findings.append(f"STEP_{n}_TOOL_CAPABILITY_UNCOVERED")
  if not tool.get("authority_bound",False):findings.append(f"STEP_{n}_AUTHORITY_UNBOUND")
  if not tool.get("verifier_bound",False):findings.append(f"STEP_{n}_VERIFIER_UNBOUND")
  if not tool.get("receipt_bound",False):findings.append(f"STEP_{n}_RECEIPT_UNBOUND")
 return findings
def compare(request,candidate_envelope):
 kind=request.get("planning_type");candidate=_candidate_result(candidate_envelope);oracle=solve(request)["result"];findings=[]
 if candidate.get("planning_type")!=kind:findings.append("PLANNING_TYPE_DISAGREEMENT")
 if bool(candidate.get("solved"))!=bool(oracle.get("solved")):findings.append("SOLVABILITY_DISAGREEMENT")
 candidate_valid=True;candidate_totals={};oracle_totals={};paths={"classical","cost_optimal","numeric","temporal","preferences","flow_constrained","multi_agent","rdf_derived"}
 if kind in paths:
  candidate_valid,candidate_totals,errors=_validate_steps(request,candidate)
  if not candidate_valid:findings.extend(errors);findings.append("CANDIDATE_PLAN_INVALID")
  oracle_valid,oracle_totals,oracle_errors=_validate_steps(request,oracle)
  if not oracle_valid:raise OracleError("ORACLE_SELF_VALIDATION_FAILED",errors=oracle_errors)
  if _objective(kind,oracle_totals)!=(_objective(kind,candidate_totals) if candidate_valid else None):findings.append("OBJECTIVE_VALUE_DISAGREEMENT")
  if kind=="multi_agent":findings.extend(_validate_multi(request,candidate))
 elif kind in {"probabilistic","fond","contingent"}:
  if _signature(candidate)["policy"]!=_signature(oracle)["policy"]:findings.append("POLICY_DISAGREEMENT")
 elif kind in {"hierarchical","resolution_adaptive","partial_order","workflow"}:
  if _signature(candidate)["decomposition"]!=_signature(oracle)["decomposition"]:findings.append("DECOMPOSITION_DISAGREEMENT")
 elif kind=="a2a_delegated":
  if _signature(candidate)["actions"]!=_signature(oracle)["actions"]:findings.append("DECOMPOSITION_DISAGREEMENT")
  findings.extend(_validate_a2a(request,candidate))
 elif kind=="mcp_bound":
  if _signature(candidate)["actions"]!=_signature(oracle)["actions"]:findings.append("DECOMPOSITION_DISAGREEMENT")
  findings.extend(_validate_mcp(request,candidate))
 elif kind=="conformant" and _signature(candidate)["actions"]!=_signature(oracle)["actions"]:findings.append("ACTION_SEQUENCE_DISAGREEMENT")
 receipt={"schema":"urn:mfw:planner-oracle-receipt:v1","oracle":"mfw-python-v1","planning_type":kind,"request_digest":_digest(request),"candidate_digest":_digest(candidate),"oracle_result_digest":_digest(oracle),"candidate_valid":candidate_valid,"agreement":not findings,"findings":sorted(set(findings)),"candidate_objective":candidate_totals,"oracle_objective":oracle_totals}
 receipt["receipt_digest"]=_digest(receipt);return receipt
