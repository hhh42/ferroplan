"""Manifest-driven WIP=1 lifecycle controller projected from MFW df511b2."""
from __future__ import annotations
from dataclasses import dataclass
import hashlib,json,os,re,subprocess,time
from pathlib import Path
from typing import Any
SCHEMA="urn:mfw:autonomic-lifecycle-manifest:v1";RECEIPT_SCHEMA="urn:mfw:autonomic-lifecycle-receipt:v1";TERMINAL={"ALIVE","BUILD_BROKEN","BLOCKED","UNSUPPORTED","REFUSED"}
class LifecycleError(Exception):
 def __init__(self,code:str,**details:Any):super().__init__(code);self.code=code;self.details=details
@dataclass(frozen=True)
class CommandResult:
 argv:list[str];cwd:str;attempt:int;exit_code:int|None;timed_out:bool;duration_ms:int;stdout:str;stderr:str
def _canonical(v):return json.dumps(v,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode()
def _digest(v):return "sha256:"+hashlib.sha256(_canonical(v)).hexdigest()
def _text_digest(v):return "sha256:"+hashlib.sha256(v.encode()).hexdigest()
def _require(ok,code,**details):
 if not ok:raise LifecycleError(code,**details)
def _validate_command(command,stage,field):_require(isinstance(command,list) and command and all(isinstance(x,str) and x for x in command),"COMMAND_INVALID",stage=stage,field=field)
def _topological(stages):
 by_id={s["id"]:s for s in stages};remaining=set(by_id);complete=set();ordered=[]
 while remaining:
  ready=sorted(x for x in remaining if set(by_id[x].get("depends_on",[]))<=complete)
  if not ready:raise LifecycleError("DEPENDENCY_CYCLE",stages=sorted(remaining))
  for name in ready:ordered.append(by_id[name]);complete.add(name);remaining.remove(name)
 return ordered
def load_manifest(path):
 source=Path(path).resolve()
 try:value=json.loads(source.read_text(encoding="utf-8"))
 except FileNotFoundError as e:raise LifecycleError("MANIFEST_NOT_FOUND",path=str(source)) from e
 except json.JSONDecodeError as e:raise LifecycleError("MANIFEST_INVALID_JSON",line=e.lineno,column=e.colno) from e
 _require(isinstance(value,dict),"MANIFEST_NOT_OBJECT");_require(value.get("schema")==SCHEMA,"MANIFEST_SCHEMA_UNSUPPORTED",schema=value.get("schema"));_require(isinstance(value.get("id"),str) and value["id"],"MANIFEST_ID_MISSING");_require(isinstance(value.get("workspace"),str) and value["workspace"],"WORKSPACE_MISSING")
 stages=value.get("stages");_require(isinstance(stages,list) and stages,"STAGES_EMPTY");seen=set()
 for stage in stages:
  _require(isinstance(stage,dict),"STAGE_NOT_OBJECT");name=stage.get("id");_require(isinstance(name,str) and name,"STAGE_ID_MISSING");_require(name not in seen,"STAGE_ID_DUPLICATE",stage=name);seen.add(name);_validate_command(stage.get("command"),name,"command");_require(isinstance(stage.get("depends_on",[]),list),"STAGE_DEPENDENCIES_INVALID",stage=name);_require(isinstance(stage.get("timeout_seconds",300),int) and stage.get("timeout_seconds",300)>0,"TIMEOUT_INVALID",stage=name);_require(stage.get("mutation","read_only") in {"read_only","bounded_write"},"MUTATION_CLASS_INVALID",stage=name)
  repairs=stage.get("repairs",[]);_require(isinstance(repairs,list),"REPAIRS_INVALID",stage=name);repair_ids=set()
  for repair in repairs:
   rid=repair.get("id");_require(isinstance(rid,str) and rid and rid not in repair_ids,"REPAIR_ID_INVALID",stage=name,repair=rid);repair_ids.add(rid);_validate_command(repair.get("command"),name,f"repair:{rid}");match=repair.get("match",{});_require(isinstance(match,dict) and any(k in match for k in ("exit_codes","stdout_regex","stderr_regex","timed_out")),"REPAIR_MATCH_EMPTY",stage=name,repair=rid)
   for key in ("stdout_regex","stderr_regex"):
    if key in match:
     try:re.compile(match[key])
     except re.error as e:raise LifecycleError("REPAIR_REGEX_INVALID",stage=name,repair=rid,field=key,reason=str(e)) from e
 for stage in stages:
  for dep in stage.get("depends_on",[]):_require(dep in seen and dep!=stage["id"],"DEPENDENCY_INVALID",stage=stage["id"],dependency=dep)
 _topological(stages);return value
def _workspace(source,manifest):
 path=(source.parent/manifest["workspace"]).resolve();_require(path.is_dir(),"WORKSPACE_NOT_DIRECTORY",workspace=str(path));return path
def _cwd(workspace,relative):
 path=(workspace/relative).resolve();_require(path==workspace or workspace in path.parents,"CWD_ESCAPES_WORKSPACE",cwd=str(path));_require(path.is_dir(),"CWD_NOT_DIRECTORY",cwd=str(path));return path
def _environment(manifest,stage):
 allowed=set(manifest.get("inherit_environment",["PATH","HOME","TMPDIR","TEMP","TMP","CI"]));env={k:v for k,v in os.environ.items() if k in allowed}
 for source in (manifest.get("environment",{}),stage.get("environment",{})):
  _require(isinstance(source,dict),"ENVIRONMENT_INVALID",stage=stage["id"])
  for k,v in source.items():_require(isinstance(k,str) and isinstance(v,str),"ENVIRONMENT_ENTRY_INVALID",stage=stage["id"]);env[k]=v
 return env
def _execute(argv,cwd,timeout,env,attempt):
 started=time.monotonic_ns()
 try:p=subprocess.run(argv,cwd=cwd,env=env,text=True,capture_output=True,timeout=timeout,check=False,shell=False);code=p.returncode;timed=False;stdout=p.stdout;stderr=p.stderr
 except subprocess.TimeoutExpired as e:code=None;timed=True;stdout=e.stdout if isinstance(e.stdout,str) else "";stderr=e.stderr if isinstance(e.stderr,str) else ""
 except FileNotFoundError as e:code=127;timed=False;stdout="";stderr=str(e)
 return CommandResult(list(argv),str(cwd),attempt,code,timed,(time.monotonic_ns()-started)//1_000_000,stdout,stderr)
def _command_receipt(r,limit):
 def preview(text):
  raw=text.encode();return (text,False) if len(raw)<=limit else (raw[:limit].decode(errors="replace"),True)
 out,out_t=preview(r.stdout);err,err_t=preview(r.stderr)
 return {"argv":r.argv,"cwd":r.cwd,"attempt":r.attempt,"exit_code":r.exit_code,"timed_out":r.timed_out,"duration_ms":r.duration_ms,"stdout_digest":_text_digest(r.stdout),"stderr_digest":_text_digest(r.stderr),"stdout_preview":out,"stderr_preview":err,"stdout_truncated":out_t,"stderr_truncated":err_t}
def _matches(repair,result):
 match=repair["match"];tests=[]
 if "exit_codes" in match:tests.append(result.exit_code in match["exit_codes"])
 if "timed_out" in match:tests.append(result.timed_out is bool(match["timed_out"]))
 if "stdout_regex" in match:tests.append(re.search(match["stdout_regex"],result.stdout,re.MULTILINE) is not None)
 if "stderr_regex" in match:tests.append(re.search(match["stderr_regex"],result.stderr,re.MULTILINE) is not None)
 return all(tests)
def _failure(stage,result):
 if result.timed_out:return "BLOCKED"
 if result.exit_code==127:return "UNSUPPORTED"
 value=stage.get("failure_classification","BUILD_BROKEN");_require(value in TERMINAL-{"ALIVE"},"FAILURE_CLASSIFICATION_INVALID",stage=stage["id"]);return value
def run_lifecycle(manifest_path,allow_repairs=True):
 source=Path(manifest_path).resolve();manifest=load_manifest(source);workspace=_workspace(source,manifest);limit=int(manifest.get("output_preview_bytes",8192));_require(limit>=0,"OUTPUT_LIMIT_INVALID");receipts=[];standings={};lifecycle="ALIVE"
 for stage in _topological(manifest["stages"]):
  blocked=[d for d in stage.get("depends_on",[]) if standings.get(d)!="ALIVE"]
  if blocked:receipts.append({"id":stage["id"],"standing":"BLOCKED","classification":"DEPENDENCY_NOT_ALIVE","blocked_by":blocked,"commands":[],"repairs":[]});lifecycle="BLOCKED";break
  cwd=_cwd(workspace,stage.get("cwd","."));env=_environment(manifest,stage);success=set(stage.get("success_exit_codes",[0]));commands=[];repairs=[];result=_execute(stage["command"],cwd,stage.get("timeout_seconds",300),env,1);commands.append(_command_receipt(result,limit));standing="ALIVE" if not result.timed_out and result.exit_code in success else None;classification="COMMAND_SUCCEEDED" if standing else "COMMAND_FAILED"
  if standing is None and allow_repairs:
   matching=[r for r in stage.get("repairs",[]) if _matches(r,result)]
   if len(matching)>1:raise LifecycleError("REPAIR_AMBIGUOUS",stage=stage["id"],repairs=[r["id"] for r in matching])
   if matching:
    repair=matching[0];rr=_execute(repair["command"],cwd,repair.get("timeout_seconds",300),env,1);receipt=_command_receipt(rr,limit);receipt["id"]=repair["id"];repairs.append(receipt)
    if not rr.timed_out and rr.exit_code in set(repair.get("success_exit_codes",[0])):
     result=_execute(stage["command"],cwd,stage.get("timeout_seconds",300),env,2);commands.append(_command_receipt(result,limit))
     if not result.timed_out and result.exit_code in success:standing="ALIVE";classification="REPAIRED_AND_VERIFIED"
     else:classification="REPAIR_DID_NOT_CLOSE"
    else:classification="REPAIR_FAILED"
  if standing is None:standing=_failure(stage,result);lifecycle=standing
  standings[stage["id"]]=standing;receipts.append({"id":stage["id"],"standing":standing,"classification":classification,"mutation":stage.get("mutation","read_only"),"commands":commands,"repairs":repairs})
  if standing!="ALIVE":break
 receipt={"schema":RECEIPT_SCHEMA,"controller":"mfw-autonomic-lifecycle-v1","manifest_id":manifest["id"],"manifest_digest":_digest(manifest),"workspace":str(workspace),"standing":lifecycle,"wip_limit":1,"repairs_allowed":allow_repairs,"stages":receipts,"replay":{"command":["python3","-m","mfw_autonomic_lifecycle","replay",str(source)],"requires_receipt":True}}
 receipt["receipt_digest"]=_digest(receipt);return receipt
def replay(manifest_path,prior):
 source=Path(manifest_path).resolve();manifest=load_manifest(source);_require(prior.get("schema")==RECEIPT_SCHEMA,"RECEIPT_SCHEMA_UNSUPPORTED");_require(prior.get("manifest_digest")==_digest(manifest),"MANIFEST_DRIFT");_require(prior.get("receipt_digest")==_digest({k:v for k,v in prior.items() if k!="receipt_digest"}),"RECEIPT_DIGEST_INVALID");_require(all(s.get("mutation")=="read_only" for s in prior.get("stages",[])),"REPLAY_MUTATING_STAGE_REFUSED");current=run_lifecycle(source,False)
 def consequence(r):return [(s["id"],s["standing"],[(c["argv"],c["exit_code"],c["stdout_digest"],c["stderr_digest"]) for c in s["commands"]]) for s in r.get("stages",[])]
 agreement=consequence(prior)==consequence(current) and prior.get("standing")==current.get("standing");result={"schema":"urn:mfw:autonomic-lifecycle-replay:v1","controller":"mfw-autonomic-lifecycle-v1","manifest_digest":current["manifest_digest"],"prior_receipt_digest":prior["receipt_digest"],"current_receipt_digest":current["receipt_digest"],"agreement":agreement,"standing":"ALIVE" if agreement else "BUILD_BROKEN"};result["replay_digest"]=_digest(result);return result
