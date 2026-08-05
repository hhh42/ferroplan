"""Independent bounded Python oracle for Ferroplan's universal planning schema.

Projected from seanchatmangpt/mfw@df511b2dd6aec591d49bf25f652d46a2d03fc3d1.
This module deliberately shares no implementation code with Ferroplan.
"""
from __future__ import annotations
from collections import defaultdict, deque
from dataclasses import dataclass
import heapq, json
from typing import Any, Iterable
PPM=1_000_000
PLANNING_TYPES=("classical","cost_optimal","numeric","temporal","preferences","probabilistic","fond","conformant","contingent","hierarchical","partial_order","workflow","flow_constrained","resolution_adaptive","multi_agent","rdf_derived","a2a_delegated","mcp_bound")
class OracleError(Exception):
 def __init__(self,code:str,**details:Any): super().__init__(code); self.code=code; self.details=details
 def as_dict(self): return {"ok":False,"error":{"code":self.code,**self.details}}
@dataclass(frozen=True)
class Limits:
 max_depth:int=128; max_states:int=100_000; max_iterations:int=512
 @classmethod
 def from_request(cls,value):
  value=value or {}; x=cls(int(value.get("max_depth",128)),int(value.get("max_states",100_000)),int(value.get("max_iterations",512)))
  if min(x.max_depth,x.max_states,x.max_iterations)<=0: raise OracleError("INVALID_LIMITS")
  return x
def _problem(request):
 kind=request.get("planning_type")
 if kind not in PLANNING_TYPES: raise OracleError("UNKNOWN_PLANNING_TYPE",planning_type=kind)
 problem=request.get("problem")
 if not isinstance(problem,dict): raise OracleError("INVALID_PROBLEM")
 return kind,problem,Limits.from_request(request.get("limits"))
def _states(problem):
 result={}
 for item in problem.get("states",[]):
  sid=item.get("id")
  if not isinstance(sid,str) or not sid or sid in result: raise OracleError("INVALID_STATE_ID",state=sid)
  result[sid]={"id":sid,"facts":frozenset(item.get("facts",[])),"fluents":{str(k):int(v) for k,v in item.get("fluents",{}).items()}}
 return result
def _goal_holds(goal,state):
 if not set(goal.get("facts",[])).issubset(state["facts"]): return False
 return all(state["fluents"].get(n) is not None and state["fluents"][n]>=int(v) for n,v in goal.get("numeric_min",{}).items()) and all(state["fluents"].get(n) is not None and state["fluents"][n]<=int(v) for n,v in goal.get("numeric_max",{}).items())
def _transitions(problem,states):
 edges=[]; masses=defaultdict(int)
 for raw in problem.get("transitions",[]):
  edge={"action":str(raw.get("action","")),"from":str(raw.get("from","")),"to":str(raw.get("to","")),"cost":int(raw.get("cost",1)),"duration":int(raw.get("duration",1)),"reward":int(raw.get("reward",0)),"probability_ppm":int(raw.get("probability_ppm",PPM)),"observation":raw.get("observation"),"requires":frozenset(raw.get("requires",[]))}
  if not edge["action"]: raise OracleError("INVALID_ACTION")
  if edge["from"] not in states: raise OracleError("UNKNOWN_STATE",state=edge["from"])
  if edge["to"] not in states: raise OracleError("UNKNOWN_STATE",state=edge["to"])
  if edge["cost"]<0 or edge["duration"]<0: raise OracleError("NEGATIVE_WEIGHT",action=edge["action"])
  if not 0<=edge["probability_ppm"]<=PPM: raise OracleError("INVALID_PROBABILITY",action=edge["action"])
  masses[(edge["from"],edge["action"])]+=edge["probability_ppm"]; edges.append(edge)
 for (state,action),mass in masses.items():
  if mass!=PPM: raise OracleError("INVALID_PROBABILITY_MASS",state=state,action=action,mass=mass)
 return edges
def _model(problem):
 states=_states(problem); edges=_transitions(problem,states); initial=[str(s) for s in problem.get("initial_states",[])]
 for state in initial:
  if state not in states: raise OracleError("UNKNOWN_STATE",state=state)
 return states,edges,initial,problem.get("goal",{})
def _outgoing(edges):
 r=defaultdict(list)
 for e in edges:r[e["from"]].append(e)
 for g in r.values():g.sort(key=lambda e:(e["action"],e["to"],e["cost"],e["duration"]))
 return r
def _action_groups(edges):
 r=defaultdict(list)
 for e in edges:r[(e["from"],e["action"])].append(e)
 for g in r.values():g.sort(key=lambda e:(e["to"],e["probability_ppm"],e["observation"] or ""))
 return r
def _empty(kind): return {"planning_type":kind,"solved":True,"steps":[],"policy":[],"decomposition":[],"notes":[]}
def _step(e,start=0,**b): return {"action":e["action"],"from":e["from"],"to":e["to"],"start":start,"duration":e["duration"],"agent":b.get("agent"),"tool":b.get("tool")}
def _schedule(edges):
 start=0; result=[]
 for e in edges: result.append(_step(e,start)); start+=e["duration"]
 return result
def _path(problem,limits,metric):
 states,edges,initial,goal=_model(problem)
 if not initial: raise OracleError("EMPTY_INITIAL_STATE")
 outgoing=_outgoing(edges); distance={}; parent={}; heap=[]
 for sid in sorted(initial):distance[sid]=0; heapq.heappush(heap,(0,sid))
 visited=0; goal_id=None
 while heap:
  score,sid=heapq.heappop(heap)
  if distance.get(sid)!=score:continue
  visited+=1
  if visited>limits.max_states:raise OracleError("RESOURCE_BOUND",resource="states",limit=limits.max_states)
  if _goal_holds(goal,states[sid]):goal_id=sid;break
  for e in outgoing.get(sid,[]):
   w=1 if metric=="steps" else e[metric]; candidate=score+w
   if e["to"] not in distance or candidate<distance[e["to"]]:distance[e["to"]]=candidate;parent[e["to"]]=(sid,e);heapq.heappush(heap,(candidate,e["to"]))
 if goal_id is None:raise OracleError("NO_PLAN")
 rev=[]; cursor=goal_id
 while cursor in parent:
  previous,e=parent[cursor];rev.append(e);cursor=previous
 return _schedule(reversed(rev))
def _preference_path(problem,limits):
 states,edges,initial,goal=_model(problem)
 if not initial:raise OracleError("EMPTY_INITIAL_STATE")
 outgoing=_outgoing(edges); penalties={str(k):int(v) for k,v in problem.get("soft_goal_facts",{}).items()}; heap=[]
 for sid in sorted(initial):heapq.heappush(heap,(0,0,0,sid,()))
 best={}; expanded=0
 while heap:
  _,cost,depth,sid,path=heapq.heappop(heap);expanded+=1
  if expanded>limits.max_states:raise OracleError("RESOURCE_BOUND",resource="states",limit=limits.max_states)
  if _goal_holds(goal,states[sid]):return _schedule(path)
  if depth>=limits.max_depth:continue
  for e in outgoing.get(sid,[]):
   target=states[e["to"]]; penalty=sum(v for f,v in penalties.items() if f not in target["facts"]); key=(e["to"],depth+1); value=(penalty,cost+e["cost"])
   if key not in best or value<best[key]:best[key]=value;heapq.heappush(heap,(penalty,cost+e["cost"],depth+1,e["to"],path+(e,)))
 raise OracleError("NO_PLAN")
def _policy_entries(policy,groups):
 return [{"state":s,"action":a,"outcomes":[{"state":e["to"],"probability_ppm":e["probability_ppm"],"observation":e["observation"]} for e in groups[(s,a)]]} for s,a in sorted(policy.items())]
def _probabilistic(problem,limits):
 states,edges,initial,goal=_model(problem);groups=_action_groups(edges);actions=defaultdict(list)
 for s,a in groups:actions[s].append(a)
 for values in actions.values():values.sort()
 values={s:(PPM if _goal_holds(goal,x) else 0) for s,x in states.items()};policy={}
 for _ in range(limits.max_iterations):
  changed=False; nxt=dict(values)
  for sid in sorted(states):
   if _goal_holds(goal,states[sid]):continue
   candidates=[(sum(e["probability_ppm"]*values[e["to"]]//PPM for e in groups[(sid,a)]),a) for a in actions.get(sid,[])]
   if candidates:
    value,action=max(candidates,key=lambda x:(x[0],x[1]));changed|=nxt[sid]!=value or policy.get(sid)!=action;nxt[sid]=value;policy[sid]=action
  values=nxt
  if not changed:break
 if not initial or any(values.get(s,0)==0 for s in initial):raise OracleError("NO_PLAN")
 return _policy_entries(policy,groups)
def _fond(problem,limits):
 states,edges,initial,goal=_model(problem);unsafe=set(problem.get("unsafe_states",[]));groups=_action_groups(edges);winning={s for s,x in states.items() if _goal_holds(goal,x)};policy={}
 for _ in range(limits.max_iterations):
  additions=[]
  for (s,a),outcomes in sorted(groups.items()):
   targets={e["to"] for e in outcomes}
   if s not in winning and s not in unsafe and targets and targets.issubset(winning) and not targets&unsafe:additions.append((s,a))
  if not additions:break
  for s,a in additions:
   if s not in winning:winning.add(s);policy[s]=a
 if not initial or not set(initial).issubset(winning):raise OracleError("NO_PLAN")
 return _policy_entries(policy,groups)
def _belief_successors(belief,action,groups):
 targets=set()
 for sid in belief:
  outcomes=groups.get((sid,action))
  if not outcomes:return None
  targets.update(e["to"] for e in outcomes)
 return frozenset(targets)
def _conformant(problem,limits):
 states,edges,initial,goal=_model(problem)
 if not initial:raise OracleError("EMPTY_INITIAL_STATE")
 groups=_action_groups(edges);start=frozenset(initial);queue=deque([(start,())]);seen={start};expanded=0
 while queue:
  belief,acts=queue.popleft();expanded+=1
  if expanded>limits.max_states:raise OracleError("RESOURCE_BOUND",resource="belief_states",limit=limits.max_states)
  if all(_goal_holds(goal,states[s]) for s in belief):
   result=[];cursor=start;t=0
   for action in acts:
    representative=groups[(sorted(cursor)[0],action)][0];result.append(_step(representative,t));t+=representative["duration"];cursor=_belief_successors(cursor,action,groups) or frozenset()
   return result
  if len(acts)>=limits.max_depth:continue
  common=set.intersection(*(set(a for s,a in groups if s==sid) for sid in belief)) if belief else set()
  for action in sorted(common):
   successor=_belief_successors(belief,action,groups)
   if successor and successor not in seen:seen.add(successor);queue.append((successor,acts+(action,)))
 raise OracleError("NO_PLAN")
def _contingent(problem,limits):
 states,edges,initial,goal=_model(problem);groups=_action_groups(edges);memo={};visiting=set()
 def visit(belief,depth):
  if all(_goal_holds(goal,states[s]) for s in belief):memo[belief]=(True,None);return True
  if depth>=limits.max_depth or belief in visiting:return False
  if belief in memo:return memo[belief][0]
  visiting.add(belief);common=set.intersection(*(set(a for s,a in groups if s==sid) for sid in belief)) if belief else set()
  for action in sorted(common):
   partitions=defaultdict(set);applicable=True
   for sid in belief:
    outcomes=groups.get((sid,action),[])
    if not outcomes:applicable=False;break
    for e in outcomes:partitions[str(e["observation"] or "__none__")].add(e["to"])
   if applicable and partitions and all(visit(frozenset(v),depth+1) for v in partitions.values()):visiting.remove(belief);memo[belief]=(True,action);return True
  visiting.remove(belief);memo[belief]=(False,None);return False
 start=frozenset(initial)
 if not initial or not visit(start,0):raise OracleError("NO_PLAN")
 policy=[]
 for belief,(solved,action) in sorted(memo.items(),key=lambda x:tuple(sorted(x[0]))):
  if solved and action is not None:policy.append({"state":"|".join(sorted(belief)),"action":action,"outcomes":[{"state":e["to"],"probability_ppm":e["probability_ppm"],"observation":e["observation"]} for sid in sorted(belief) for e in groups[(sid,action)]]})
 return policy
def _task_index(problem):
 tasks={}
 for task in problem.get("tasks",[]):
  tid=str(task.get("id",""))
  if not tid or tid in tasks:raise OracleError("INVALID_TASK",task=tid)
  tasks[tid]={"id":tid,"primitive_action":task.get("primitive_action"),"requires":frozenset(task.get("requires",[]))}
 return tasks
def _decompose(problem,limits):
 tasks=_task_index(problem);methods=defaultdict(list)
 for m in problem.get("methods",[]):methods[str(m.get("task",""))].append(m)
 for v in methods.values():v.sort(key=lambda m:str(m.get("id","")))
 actions=[];leaves=[]
 def visit(tid,stack):
  if len(stack)>=limits.max_depth:raise OracleError("RESOURCE_BOUND",resource="depth",limit=limits.max_depth)
  if tid in stack:raise OracleError("HIERARCHY_CYCLE",task=tid)
  task=tasks.get(tid)
  if task is None:raise OracleError("UNKNOWN_TASK",task=tid)
  if task["primitive_action"] is not None:actions.append(str(task["primitive_action"]));leaves.append(task);return
  choices=methods.get(tid,[])
  if not choices:raise OracleError("NO_METHOD",task=tid)
  last=None
  for m in choices:
   checkpoint=(len(actions),len(leaves))
   try:
    for child in m.get("subtasks",[]):visit(str(child),stack+(tid,))
    return
   except OracleError as e:del actions[checkpoint[0]:];del leaves[checkpoint[1]:];last=e
  raise last or OracleError("NO_METHOD",task=tid)
 roots=[str(x) for x in problem.get("root_tasks",[])] or sorted(tasks)
 for root in roots:visit(root,())
 return actions,leaves
def _workflow(problem):
 tasks=_task_index(problem);indegree={x:0 for x in tasks};outgoing=defaultdict(list)
 for e in problem.get("workflow_edges",[]):
  before,after=str(e.get("before","")),str(e.get("after",""))
  if before not in tasks:raise OracleError("UNKNOWN_TASK",task=before)
  if after not in tasks:raise OracleError("UNKNOWN_TASK",task=after)
  outgoing[before].append(after);indegree[after]+=1
 heap=[x for x,d in indegree.items() if d==0];heapq.heapify(heap);order=[]
 while heap:
  tid=heapq.heappop(heap);order.append(tid)
  for child in sorted(outgoing.get(tid,[])):
   indegree[child]-=1
   if indegree[child]==0:heapq.heappush(heap,child)
 if len(order)!=len(tasks):raise OracleError("WORKFLOW_CYCLE")
 return order
def _assign_agents(steps,requirements,problem):
 agents=[{"id":str(a.get("id","")),"capabilities":frozenset(a.get("capabilities",[])),"remaining":int(a.get("capacity",1))-int(a.get("current_wip",0))} for a in problem.get("agents",[])]
 for step,required in zip(steps,requirements,strict=True):
  candidates=[a for a in agents if a["remaining"]>0 and required.issubset(a["capabilities"])]
  if not candidates:raise OracleError("CAPABILITY_UNCOVERED",item=step["action"],missing=sorted(required))
  chosen=min(candidates,key=lambda a:(-a["remaining"],a["id"]));step["agent"]=chosen["id"];chosen["remaining"]-=1
def _bind_tools(steps,requirements,problem):
 tools=sorted(problem.get("tools",[]),key=lambda t:str(t.get("id","")))
 for step,required in zip(steps,requirements,strict=True):
  capable=[t for t in tools if required.issubset(frozenset(t.get("capabilities",[])))]
  if not capable:raise OracleError("CAPABILITY_UNCOVERED",item=step["action"],missing=sorted(required))
  for t in capable:
   if not t.get("authority_bound",False):raise OracleError("AUTHORITY_UNBOUND",tool=str(t.get("id","")))
   if not t.get("verifier_bound",False):raise OracleError("VERIFIER_UNBOUND",tool=str(t.get("id","")))
   if not t.get("receipt_bound",False):raise OracleError("RECEIPT_UNBOUND",tool=str(t.get("id","")))
  step["tool"]=str(capable[0].get("id",""))
def _rdf_project(problem):
 states=set();initial=set();goals=set();attrs=defaultdict(dict)
 for t in problem.get("rdf",[]):
  s,p,o=str(t.get("subject","")),str(t.get("predicate","")),str(t.get("object",""))
  if p=="state" and o=="true":states.add(s)
  elif p=="initial" and o=="true":initial.add(s)
  elif p=="goal" and o=="true":goals.add(s)
  else:attrs[s][p]=o
 if not states or not initial or not goals:raise OracleError("INVALID_RDF_PROJECTION",reason="missing state/initial/goal")
 transitions=[{"action":v.get("action",s),"from":v["from"],"to":v["to"],"cost":int(v.get("cost",1)),"duration":int(v.get("duration",1)),"probability_ppm":int(v.get("probability_ppm",PPM))} for s,v in sorted(attrs.items()) if {"from","to"}.issubset(v)]
 return {"states":[{"id":s,"facts":["__rdf_goal__"] if s in goals else [],"fluents":{}} for s in sorted(states)],"initial_states":sorted(initial),"goal":{"facts":["__rdf_goal__"]},"transitions":transitions}
def solve(request):
 kind,problem,limits=_problem(request);result=_empty(kind)
 if kind in {"classical","numeric"}:result["steps"]=_path(problem,limits,"steps" if kind=="classical" else "cost")
 elif kind=="cost_optimal":result["steps"]=_path(problem,limits,"cost")
 elif kind=="temporal":result["steps"]=_path(problem,limits,"duration")
 elif kind=="preferences":result["steps"]=_preference_path(problem,limits)
 elif kind=="probabilistic":result["policy"]=_probabilistic(problem,limits)
 elif kind=="fond":result["policy"]=_fond(problem,limits)
 elif kind=="conformant":result["steps"]=_conformant(problem,limits)
 elif kind=="contingent":result["policy"]=_contingent(problem,limits)
 elif kind in {"hierarchical","resolution_adaptive"}:result["decomposition"]=_decompose(problem,limits)[0]
 elif kind in {"partial_order","workflow"}:result["decomposition"]=_workflow(problem)
 elif kind=="flow_constrained":
  for q in problem.get("queues",[]):
   current,maximum=int(q.get("current_wip",0)),int(q.get("max_wip",0))
   if current>=maximum:raise OracleError("WIP_BOUND_EXCEEDED",queue=str(q.get("id","")),current=current,max=maximum)
  result["steps"]=_path(problem,limits,"cost")
 elif kind=="multi_agent":
  result["steps"]=_path(problem,limits,"cost");edge_by_key={(e["action"],e["from"],e["to"]):e for e in _transitions(problem,_states(problem))};_assign_agents(result["steps"],[edge_by_key[(s["action"],s["from"],s["to"])]["requires"] for s in result["steps"]],problem)
 elif kind=="rdf_derived":result["steps"]=_path(_rdf_project(problem),limits,"cost")
 elif kind in {"a2a_delegated","mcp_bound"}:
  actions,leaves=_decompose(problem,limits);result["decomposition"]=actions;result["steps"]=[{"action":a,"from":None,"to":None,"start":i,"duration":1,"agent":None,"tool":None} for i,a in enumerate(actions)];requirements=[x["requires"] for x in leaves]
  (_assign_agents if kind=="a2a_delegated" else _bind_tools)(result["steps"],requirements,problem)
 else:raise OracleError("UNKNOWN_PLANNING_TYPE",planning_type=kind)
 return {"ok":True,"oracle":"mfw-python-v1","result":result}
def solve_json(text):
 try:
  request=json.loads(text)
  if not isinstance(request,dict):raise OracleError("INVALID_REQUEST")
  result=solve(request)
 except json.JSONDecodeError as e:result=OracleError("INVALID_JSON",line=e.lineno,column=e.colno).as_dict()
 except OracleError as e:result=e.as_dict()
 return json.dumps(result,sort_keys=True,separators=(",",":"))
