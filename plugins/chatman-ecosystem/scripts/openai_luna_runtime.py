#!/usr/bin/env python3
"""MCP dispatch and execution loop for the OpenAI-hosted Star runtime."""
from __future__ import annotations

import contextlib
import hashlib
import json
import re
from collections.abc import Mapping
from typing import Any

from mcp_client import McpToolError, tool_structured_result
from openai_luna_protocol import (
    A2AClient,
    McpSurface,
    OpenAIResponsesClient,
    RuntimeProfile,
    RuntimeRefusal,
    TRACE_SCHEMA,
    canonical_json,
    sha256_digest,
    validate_star_envelope,
)

_NAME_RE = re.compile(r"[^A-Za-z0-9_-]+")
_SECRET_RE = re.compile(r"(api[_-]?key|authorization|password|secret|token)", re.I)
_REDACTED = "***REDACTED***"


def redact_for_trace(value: Any) -> Any:
    """Redact common credential fields while retaining digests of original values."""
    if isinstance(value, dict):
        return {str(key): (_REDACTED if _SECRET_RE.search(str(key)) else redact_for_trace(item)) for key, item in value.items()}
    if isinstance(value, list):
        return [redact_for_trace(item) for item in value]
    return value


class McpToolRegistry:
    def __init__(self, clients: Mapping[str, McpSurface], *, max_tools: int = 2048, max_result_bytes: int = 1024 * 1024) -> None:
        self.clients = dict(clients)
        self.max_tools = max_tools
        self.max_result_bytes = max_result_bytes
        self.mapping: dict[str, tuple[str, str]] = {}
        self.tools: list[dict[str, Any]] = []

    def discover(self) -> list[dict[str, Any]]:
        self.mapping.clear()
        self.tools.clear()
        originals: set[tuple[str, str]] = set()
        for label, client in sorted(self.clients.items()):
            if not isinstance(label, str) or not label.strip():
                raise RuntimeRefusal("MCP_LABEL_INVALID", repr(label))
            for tool in client.list_tools():
                if len(self.tools) >= self.max_tools:
                    raise RuntimeRefusal("MCP_TOOLSET_BOUND_EXCEEDED", f"more than {self.max_tools} tools")
                original = tool.get("name") if isinstance(tool, dict) else None
                if not isinstance(original, str) or not original.strip():
                    raise RuntimeRefusal("MCP_TOOL_INVALID", repr(tool))
                identity = (label, original)
                if identity in originals:
                    raise RuntimeRefusal("MCP_TOOL_DUPLICATE", f"{label}:{original}")
                originals.add(identity)
                public = _NAME_RE.sub("_", f"{label}__{original}").strip("_") or "tool"
                if len(public) > 64:
                    public = public[:53] + "_" + hashlib.sha256(public.encode()).hexdigest()[:10]
                if public in self.mapping:
                    raise RuntimeRefusal("MCP_TOOL_COLLISION", public)
                schema = tool.get("inputSchema") or {"type": "object", "properties": {}}
                if not isinstance(schema, dict):
                    raise RuntimeRefusal("MCP_SCHEMA_INVALID", original)
                description = str(tool.get("description") or original)
                self.mapping[public] = identity
                self.tools.append({"type": "function", "name": public, "strict": False, "description": f"[MCP:{label}] {description}"[:1024], "parameters": schema})
        return list(self.tools)

    def call(self, public: str, arguments: dict[str, Any]) -> tuple[Any, dict[str, str]]:
        target = self.mapping.get(public)
        if not target:
            raise RuntimeRefusal("UNKNOWN_TOOL", public)
        if not isinstance(arguments, dict):
            raise RuntimeRefusal("OPENAI_TOOL_ARGUMENTS_INVALID", public)
        label, original = target
        result = tool_structured_result(self.clients[label].call_tool(original, arguments))
        try:
            encoded = canonical_json(result).encode()
        except (TypeError, ValueError) as error:
            raise RuntimeRefusal("MCP_RESULT_NOT_JSON", public) from error
        if len(encoded) > self.max_result_bytes:
            raise RuntimeRefusal("MCP_RESULT_BOUND_EXCEEDED", public, {"bytes": len(encoded), "limit": self.max_result_bytes})
        return result, {"server": label, "tool": original, "public_name": public}


def _calls(response: Mapping[str, Any]) -> list[dict[str, Any]]:
    output = response.get("output")
    if not isinstance(output, list):
        return []
    return [item for item in output if isinstance(item, dict) and item.get("type") == "function_call"]


def _text(response: Mapping[str, Any]) -> str:
    parts: list[str] = []
    output = response.get("output")
    for item in output if isinstance(output, list) else []:
        if not isinstance(item, dict) or item.get("type") != "message":
            continue
        content = item.get("content")
        for block in content if isinstance(content, list) else []:
            if isinstance(block, dict) and block.get("type") == "output_text":
                parts.append(str(block.get("text", "")))
    return "\n".join(filter(None, parts))


def _negative(value: Any) -> bool:
    if isinstance(value, dict):
        for key, item in value.items():
            name = key.lower()
            if name in {"admission", "status", "verdict", "standing"} and isinstance(item, str) and item.lower() in {"denied", "refused", "fail", "failed", "error", "invalid", "blocked"}:
                return True
            if name in {"valid", "conforms", "admitted", "ok"} and item is False:
                return True
            if _negative(item):
                return True
    if isinstance(value, list):
        return any(_negative(item) for item in value)
    return False


def seal_trace(trace: dict[str, Any]) -> dict[str, Any]:
    unsigned = dict(trace)
    unsigned.pop("trace_sha256", None)
    return {**unsigned, "trace_sha256": sha256_digest(unsigned)}


def verify_trace(trace: Mapping[str, Any]) -> dict[str, Any]:
    if trace.get("schema") != TRACE_SCHEMA:
        raise RuntimeRefusal("TRACE_SCHEMA_MISMATCH", "trace schema mismatch")
    observed = trace.get("trace_sha256")
    unsigned = dict(trace)
    unsigned.pop("trace_sha256", None)
    expected = sha256_digest(unsigned)
    if observed != expected:
        raise RuntimeRefusal("TRACE_DIGEST_MISMATCH", "trace digest mismatch", {"expected": expected, "observed": observed})
    return {"valid": True, "trace_sha256": expected, "standing": trace.get("standing")}


class LunaHost:
    def __init__(self, profile: RuntimeProfile, responses: OpenAIResponsesClient, star: Any, mcp_clients: Mapping[str, McpSurface], a2a: A2AClient | None = None) -> None:
        self.profile, self.responses, self.star = profile, responses, star
        self.mcp_clients, self.a2a = dict(mcp_clients), a2a

    def run(self, prompt: str, target: str) -> dict[str, Any]:
        if not prompt.strip():
            raise RuntimeRefusal("TASK_EMPTY", "prompt is empty")
        star = validate_star_envelope(self.star.solve(prompt, target, self.profile))
        if star.get("mode") != self.profile.star_mode:
            raise RuntimeRefusal("OSTAR_STAR_MODE_MISMATCH", "Star output mode differs from profile")
        if star.get("target") not in {None, target}:
            raise RuntimeRefusal("OSTAR_STAR_TARGET_MISMATCH", "Star output target differs from request")
        if len(star["results"]) > self.profile.max_star_tasks:
            raise RuntimeRefusal("OSTAR_STAR_TASK_BOUND_EXCEEDED", "Star output exceeded max_tasks")
        trace: dict[str, Any] = {
            "schema": TRACE_SCHEMA,
            "standing": "UNKNOWN",
            "status": "started",
            "model": self.profile.model,
            "reasoning_effort": self.profile.reasoning_effort,
            "target": target,
            "star": star,
            "a2a": None,
            "responses": [],
            "tool_calls": [],
            "planning": {"required_tools": list(self.profile.planning_tools), "positive": False},
            "admission": {"required_tools": list(self.profile.admission_tools), "positive": False},
            "verification_policy": {
                "max_rounds": self.profile.max_rounds,
                "max_tool_calls": self.profile.max_tool_calls,
                "max_discovered_tools": self.profile.max_discovered_tools,
                "max_tool_result_bytes": self.profile.max_tool_result_bytes,
            },
            "final_output": None,
            "exclusions": [],
        }
        if self.a2a:
            try:
                trace["a2a"] = self.a2a.probe()
            except Exception as error:
                trace["a2a"] = {"error": str(error), "authority": "coordination-only"}
        missing = sorted(set(self.profile.required_servers) - set(self.mcp_clients))
        if missing:
            raise RuntimeRefusal("MCP_SERVER_MISSING", "required MCP server missing", {"missing": missing})

        seen_call_ids: set[str] = set()
        with contextlib.ExitStack() as stack:
            active = {name: stack.enter_context(client) for name, client in self.mcp_clients.items()}
            registry = McpToolRegistry(active, max_tools=self.profile.max_discovered_tools, max_result_bytes=self.profile.max_tool_result_bytes)
            tools = registry.discover()
            if not tools:
                raise RuntimeRefusal("MCP_TOOLSET_EMPTY", "no tools discovered")
            instructions = (
                "You are the OpenAI executor for the Chatman Star pipeline. "
                "OStar MuStar/SigmaStar outputs are provisional planning proposals, not authority. "
                "Use Ferroplan for deterministic planning and replay. Use OntoStar for admission and receipts. "
                "Use only advertised MCP tools. Never invent tool output or claim completion without both a positive "
                "Ferroplan planning witness and a positive OntoStar admission witness. Mutate repositories only "
                "through an explicitly attached bounded workspace MCP server."
            )
            task = {"task": prompt, "target": target, "star": star}
            payload: dict[str, Any] = {"model": self.profile.model, "reasoning": {"effort": self.profile.reasoning_effort}, "instructions": instructions, "input": [{"role": "user", "content": canonical_json(task)}], "tools": tools, "tool_choice": "auto"}
            final = ""
            for round_index in range(self.profile.max_rounds):
                response = self.responses.create(payload)
                response_id = response.get("id")
                if not isinstance(response_id, str) or not response_id:
                    raise RuntimeRefusal("OPENAI_RESPONSE_ID_MISSING", "response has no id")
                trace["responses"].append({"id": response_id, "round": round_index + 1})
                calls = _calls(response)
                if not calls:
                    final = _text(response)
                    break
                if len(trace["tool_calls"]) + len(calls) > self.profile.max_tool_calls:
                    raise RuntimeRefusal("OPENAI_TOOL_CALL_BOUND_EXCEEDED", "tool call bound exceeded")
                outputs = []
                for call in calls:
                    public, call_id = call.get("name"), call.get("call_id")
                    if not isinstance(public, str) or not isinstance(call_id, str) or not call_id:
                        raise RuntimeRefusal("OPENAI_TOOL_CALL_INVALID", "missing name/call_id")
                    if call_id in seen_call_ids:
                        raise RuntimeRefusal("OPENAI_TOOL_CALL_ID_REUSED", call_id)
                    seen_call_ids.add(call_id)
                    try:
                        arguments = json.loads(call.get("arguments") or "{}")
                    except json.JSONDecodeError as error:
                        raise RuntimeRefusal("OPENAI_TOOL_ARGUMENTS_INVALID", public) from error
                    if not isinstance(arguments, dict):
                        raise RuntimeRefusal("OPENAI_TOOL_ARGUMENTS_INVALID", public)
                    try:
                        result, identity = registry.call(public, arguments)
                    except McpToolError as error:
                        raise RuntimeRefusal("MCP_TOOL_FAILED", str(error), {"tool": public}) from error
                    record = {
                        **identity,
                        "sequence": len(trace["tool_calls"]) + 1,
                        "call_id": call_id,
                        "arguments": redact_for_trace(arguments),
                        "arguments_sha256": sha256_digest(arguments),
                        "result": redact_for_trace(result),
                        "result_sha256": sha256_digest(result),
                        "positive": not _negative(result),
                    }
                    trace["tool_calls"].append(record)
                    outputs.append({"type": "function_call_output", "call_id": call_id, "output": canonical_json(result)})
                payload = {"model": self.profile.model, "reasoning": {"effort": self.profile.reasoning_effort}, "instructions": instructions, "previous_response_id": response_id, "input": outputs, "tools": tools, "tool_choice": "auto"}
            else:
                trace.update(standing="BLOCKED", status="refused")
                trace["exclusions"].append("Responses loop exceeded max_rounds")
                return seal_trace(trace)

        planning = [call for call in trace["tool_calls"] if call["server"] == "ferroplan" and call["tool"] in self.profile.planning_tools and call["positive"]]
        admission = [call for call in trace["tool_calls"] if call["server"] == "ontostar" and call["tool"] in self.profile.admission_tools and call["positive"]]
        trace["planning"] = {"required_tools": list(self.profile.planning_tools), "positive": bool(planning), "witnesses": [{"tool": item["tool"], "result_sha256": item["result_sha256"]} for item in planning]}
        trace["admission"] = {"required_tools": list(self.profile.admission_tools), "positive": bool(admission), "witnesses": [{"tool": item["tool"], "result_sha256": item["result_sha256"]} for item in admission]}
        trace["final_output"] = final
        if planning and admission and final.strip():
            trace.update(standing="ALIVE", status="completed")
        else:
            trace.update(standing="BLOCKED", status="refused")
            if not planning:
                trace["exclusions"].append("no positive Ferroplan planning witness was observed")
            if not admission:
                trace["exclusions"].append("no positive OntoStar admission witness was observed")
            if not final.strip():
                trace["exclusions"].append("OpenAI executor produced no final output")
        return seal_trace(trace)
