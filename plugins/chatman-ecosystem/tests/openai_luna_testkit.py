from __future__ import annotations

import json
import stat
import sys
from pathlib import Path
from typing import Any

SCRIPTS = Path(__file__).resolve().parents[1] / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from openai_luna_protocol import (  # noqa: E402
    LUNA_MODEL,
    OSTAR_STAR_SCHEMA,
    PROFILE_SCHEMA,
    RuntimeProfile,
)


def mustar_result(**overrides: Any) -> dict[str, Any]:
    value = {
        "title": "Ferroplan task",
        "domain": "SYSTEM_DESIGN",
        "build_order": "-> (explore: [a], plan: [b], write: [c])",
        "artifact": "candidate",
        "artifact_type": "architecture_spec",
        "operator_notation": "compile-artifact",
        "build_order_adhered": True,
        "implementation_complete": True,
        "powl_model": "SEQ(a,b,c)",
        "sequence_diagram": "flowchart TD; a-->b",
    }
    value.update(overrides)
    return value


def star_envelope(**overrides: Any) -> dict[str, Any]:
    value = {
        "schema": OSTAR_STAR_SCHEMA,
        "mode": "mustar",
        "target": "ferroplan",
        "provisional": True,
        "authority": "proposer",
        "internal_actuation": False,
        "star_classes": {"planner": "MuStarPlanner", "executor": "MuStarExecutor"},
        "results": [mustar_result()],
    }
    value.update(overrides)
    return value


def profile(**overrides: Any) -> RuntimeProfile:
    value: dict[str, Any] = {
        "schema": PROFILE_SCHEMA,
        "model": LUNA_MODEL,
        "reasoning_effort": "medium",
        "max_rounds": 4,
        "max_tool_calls": 32,
        "max_discovered_tools": 128,
        "max_tool_result_bytes": 65536,
        "required_servers": ["ferroplan", "ontostar"],
        "planning_tools": ["solve"],
        "admission_tools": ["onto_admit_work_order"],
        "star": {"mode": "mustar", "domain": "SYSTEM_DESIGN", "max_tasks": 4},
    }
    value.update(overrides)
    return RuntimeProfile.from_dict(value)


class FakeStar:
    def __init__(self, value: dict[str, Any] | None = None) -> None:
        self.value = value or star_envelope()
        self.calls: list[tuple[str, str, RuntimeProfile]] = []

    def solve(self, prompt: str, target: str, runtime_profile: RuntimeProfile) -> dict[str, Any]:
        self.calls.append((prompt, target, runtime_profile))
        return self.value


class FakeMcp:
    def __init__(self, tools: list[dict[str, Any]], results: dict[str, Any], *, enter_error: Exception | None = None) -> None:
        self.tools = tools
        self.results = results
        self.enter_error = enter_error
        self.calls: list[tuple[str, dict[str, Any]]] = []
        self.entered = 0
        self.exited = 0

    def __enter__(self):
        self.entered += 1
        if self.enter_error:
            raise self.enter_error
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.exited += 1

    def list_tools(self) -> list[dict[str, Any]]:
        return self.tools

    def call_tool(self, name: str, arguments: dict[str, Any]) -> dict[str, Any]:
        self.calls.append((name, arguments))
        result = self.results[name]
        if isinstance(result, Exception):
            raise result
        return {"structuredContent": result}


class ScriptedResponses:
    def __init__(self, responses: list[dict[str, Any]]) -> None:
        self.responses = list(responses)
        self.payloads: list[dict[str, Any]] = []

    def create(self, payload: dict[str, Any]) -> dict[str, Any]:
        self.payloads.append(payload)
        if not self.responses:
            raise AssertionError("no scripted response remains")
        return self.responses.pop(0)


def function_call(response_id: str, *calls: tuple[str, str, dict[str, Any]]) -> dict[str, Any]:
    return {
        "id": response_id,
        "output": [
            {"type": "function_call", "call_id": call_id, "name": name, "arguments": json.dumps(arguments)}
            for call_id, name, arguments in calls
        ],
    }


def final_response(response_id: str = "resp-final", text: str = "done") -> dict[str, Any]:
    return {"id": response_id, "output": [{"type": "message", "content": [{"type": "output_text", "text": text}]}]}


def standard_surfaces(*, planning: Any | None = None, admission: Any | None = None) -> dict[str, FakeMcp]:
    return {
        "ferroplan": FakeMcp(
            [{"name": "solve", "description": "Solve", "inputSchema": {"type": "object"}}],
            {"solve": planning if planning is not None else {"status": "solved", "plan": ["step"]}},
        ),
        "ontostar": FakeMcp(
            [{"name": "onto_admit_work_order", "description": "Admit", "inputSchema": {"type": "object"}}],
            {"onto_admit_work_order": admission if admission is not None else {"admission": "admitted", "receipt_hash": "abc"}},
        ),
    }


def standard_script() -> list[dict[str, Any]]:
    return [
        function_call(
            "resp-1",
            ("f1", "ferroplan__solve", {}),
            ("o1", "ontostar__onto_admit_work_order", {}),
        ),
        final_response(),
    ]


def write_executable(path: Path, body: str) -> Path:
    path.write_text(body, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


def fake_mcp_server(path: Path, label: str, *, mode: str = "normal") -> Path:
    tool = "solve" if label == "ferroplan" else "onto_admit_work_order"
    result = {"status": "solved", "plan": ["step"]} if label == "ferroplan" else {"admission": "admitted", "receipt_hash": "abc"}
    body = f'''#!/usr/bin/env python3
import json, sys, time
TOOL={tool!r}
RESULT={result!r}
MODE={mode!r}
for raw in sys.stdin:
    msg=json.loads(raw)
    if "id" not in msg: continue
    method=msg.get("method")
    if MODE=="hang": time.sleep(10)
    if MODE=="malformed": print("not-json", flush=True); continue
    if MODE=="crash": sys.exit(3)
    if method=="initialize": out={{"protocolVersion":"2024-11-05","capabilities":{{}},"serverInfo":{{"name":"fake","version":"1"}}}}
    elif method=="tools/list": out={{"tools":[{{"name":TOOL,"description":TOOL,"inputSchema":{{"type":"object"}}}}]}}
    elif method=="tools/call": out={{"structuredContent":RESULT}}
    else: out={{}}
    print(json.dumps({{"jsonrpc":"2.0","id":msg["id"],"result":out}}), flush=True)
'''
    path.write_text(body, encoding="utf-8")
    path.chmod(0o644)
    return path


def fake_star_launcher(path: Path, *, internal_actuation: bool = False, malformed: bool = False) -> Path:
    if malformed:
        return write_executable(path, "#!/usr/bin/env python3\nprint('not-json')\n")
    body = f'''#!/usr/bin/env python3
import json, sys
req=json.load(sys.stdin)
out={{
"schema":{OSTAR_STAR_SCHEMA!r},"mode":req["mode"],"target":req["target"],
"provisional":True,"authority":"proposer","internal_actuation":{internal_actuation!r},
"star_classes":{{"planner":"MuStarPlanner","executor":"MuStarExecutor"}},
"results":[{mustar_result()!r}]
}}
print(json.dumps(out))
'''
    return write_executable(path, body)
