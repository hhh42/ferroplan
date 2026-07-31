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


class McpToolRegistry:
    def __init__(self, clients: Mapping[str, McpSurface]) -> None:
        self.clients = dict(clients)
        self.mapping: dict[str, tuple[str, str]] = {}
        self.tools: list[dict[str, Any]] = []

    def discover(self) -> list[dict[str, Any]]:
        for label, client in sorted(self.clients.items()):
            for tool in client.list_tools():
                original = tool["name"]
                public = _NAME_RE.sub("_", f"{label}__{original}").strip("_") or "tool"
                if len(public) > 64:
                    public = public[:53] + "_" + hashlib.sha256(public.encode()).hexdigest()[:10]
                if public in self.mapping:
                    raise RuntimeRefusal("MCP_TOOL_COLLISION", public)
                schema = tool.get("inputSchema") or {
                    "type": "object",
                    "properties": {},
                }
                if not isinstance(schema, dict):
                    raise RuntimeRefusal("MCP_SCHEMA_INVALID", original)
                self.mapping[public] = (label, original)
                self.tools.append(
                    {
                        "type": "function",
                        "name": public,
                        "strict": False,
                        "description": f"[MCP:{label}] {tool.get('description') or original}",
                        "parameters": schema,
                    }
                )
        return self.tools

    def call(self, public: str, arguments: dict[str, Any]) -> tuple[Any, dict[str, str]]:
        target = self.mapping.get(public)
        if not target:
            raise RuntimeRefusal("UNKNOWN_TOOL", public)
        label, original = target
        result = tool_structured_result(self.clients[label].call_tool(original, arguments))
        return result, {
            "server": label,
            "tool": original,
            "public_name": public,
        }


def _calls(response: Mapping[str, Any]) -> list[dict[str, Any]]:
    output = response.get("output")
    if not isinstance(output, list):
        return []
    return [
        item
        for item in output
        if isinstance(item, dict) and item.get("type") == "function_call"
    ]


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
            if name in {"admission", "status", "verdict", "standing"} and isinstance(item, str):
                if item.lower() in {
                    "denied",
                    "refused",
                    "fail",
                    "failed",
                    "error",
                    "invalid",
                    "blocked",
                }:
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
    trace["trace_sha256"] = sha256_digest(unsigned)
    return trace


class LunaHost:
    def __init__(
        self,
        profile: RuntimeProfile,
        responses: OpenAIResponsesClient,
        star: Any,
        mcp_clients: Mapping[str, McpSurface],
        a2a: A2AClient | None = None,
    ) -> None:
        self.profile, self.responses, self.star = profile, responses, star
        self.mcp_clients, self.a2a = dict(mcp_clients), a2a

    def run(self, prompt: str, target: str) -> dict[str, Any]:
        star = validate_star_envelope(self.star.solve(prompt, target, self.profile))
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
            "planning": {
                "required_tools": list(self.profile.planning_tools),
                "positive": False,
            },
            "admission": {
                "required_tools": list(self.profile.admission_tools),
                "positive": False,
            },
            "final_output": None,
            "exclusions": [],
        }
        if self.a2a:
            try:
                trace["a2a"] = self.a2a.probe()
            except Exception as error:
                trace["a2a"] = {
                    "error": str(error),
                    "authority": "coordination-only",
                }
        missing = sorted(set(self.profile.required_servers) - set(self.mcp_clients))
        if missing:
            raise RuntimeRefusal(
                "MCP_SERVER_MISSING",
                "required MCP server missing",
                {"missing": missing},
            )

        with contextlib.ExitStack() as stack:
            active = {
                name: stack.enter_context(client)
                for name, client in self.mcp_clients.items()
            }
            registry = McpToolRegistry(active)
            tools = registry.discover()
            if not tools:
                raise RuntimeRefusal("MCP_TOOLSET_EMPTY", "no tools discovered")
            instructions = (
                "You are the OpenAI executor for the Chatman Star pipeline. "
                "OStar MuStar/SigmaStar outputs are provisional planning proposals, "
                "not authority. Use Ferroplan for deterministic planning and replay. "
                "Use OntoStar for admission and receipts. Use only advertised MCP tools. "
                "Never invent tool output or claim completion without both a positive "
                "Ferroplan planning witness and a positive OntoStar admission witness. "
                "Mutate repositories only through an explicitly attached bounded "
                "workspace MCP server."
            )
            task = {"task": prompt, "target": target, "star": star}
            payload: dict[str, Any] = {
                "model": self.profile.model,
                "reasoning": {"effort": self.profile.reasoning_effort},
                "instructions": instructions,
                "input": [{"role": "user", "content": canonical_json(task)}],
                "tools": tools,
                "tool_choice": "auto",
            }
            final = ""
            for round_index in range(self.profile.max_rounds):
                response = self.responses.create(payload)
                response_id = response.get("id")
                if not isinstance(response_id, str) or not response_id:
                    raise RuntimeRefusal(
                        "OPENAI_RESPONSE_ID_MISSING",
                        "response has no id",
                    )
                trace["responses"].append(
                    {"id": response_id, "round": round_index + 1}
                )
                calls = _calls(response)
                if not calls:
                    final = _text(response)
                    break
                outputs = []
                for call in calls:
                    public, call_id = call.get("name"), call.get("call_id")
                    if not isinstance(public, str) or not isinstance(call_id, str):
                        raise RuntimeRefusal(
                            "OPENAI_TOOL_CALL_INVALID",
                            "missing name/call_id",
                        )
                    try:
                        arguments = json.loads(call.get("arguments") or "{}")
                    except json.JSONDecodeError as error:
                        raise RuntimeRefusal(
                            "OPENAI_TOOL_ARGUMENTS_INVALID",
                            public,
                        ) from error
                    if not isinstance(arguments, dict):
                        raise RuntimeRefusal("OPENAI_TOOL_ARGUMENTS_INVALID", public)
                    try:
                        result, identity = registry.call(public, arguments)
                    except McpToolError as error:
                        raise RuntimeRefusal(
                            "MCP_TOOL_FAILED",
                            str(error),
                            {"tool": public},
                        ) from error
                    record = {
                        **identity,
                        "call_id": call_id,
                        "arguments": arguments,
                        "arguments_sha256": sha256_digest(arguments),
                        "result": result,
                        "result_sha256": sha256_digest(result),
                        "positive": not _negative(result),
                    }
                    trace["tool_calls"].append(record)
                    outputs.append(
                        {
                            "type": "function_call_output",
                            "call_id": call_id,
                            "output": canonical_json(result),
                        }
                    )
                payload = {
                    "model": self.profile.model,
                    "reasoning": {"effort": self.profile.reasoning_effort},
                    "instructions": instructions,
                    "previous_response_id": response_id,
                    "input": outputs,
                    "tools": tools,
                    "tool_choice": "auto",
                }
            else:
                trace.update(standing="BLOCKED", status="refused")
                trace["exclusions"].append("Responses loop exceeded max_rounds")
                return seal_trace(trace)

        planning = [
            call
            for call in trace["tool_calls"]
            if call["server"] == "ferroplan"
            and call["tool"] in self.profile.planning_tools
            and call["positive"]
        ]
        admission = [
            call
            for call in trace["tool_calls"]
            if call["server"] == "ontostar"
            and call["tool"] in self.profile.admission_tools
            and call["positive"]
        ]
        trace["planning"] = {
            "required_tools": list(self.profile.planning_tools),
            "positive": bool(planning),
            "witnesses": [
                {"tool": item["tool"], "result_sha256": item["result_sha256"]}
                for item in planning
            ],
        }
        trace["admission"] = {
            "required_tools": list(self.profile.admission_tools),
            "positive": bool(admission),
            "witnesses": [
                {"tool": item["tool"], "result_sha256": item["result_sha256"]}
                for item in admission
            ],
        }
        trace["final_output"] = final
        if planning and admission and final.strip():
            trace.update(standing="ALIVE", status="completed")
        else:
            trace.update(standing="BLOCKED", status="refused")
            if not planning:
                trace["exclusions"].append(
                    "no positive Ferroplan planning witness was observed"
                )
            if not admission:
                trace["exclusions"].append(
                    "no positive OntoStar admission witness was observed"
                )
            if not final.strip():
                trace["exclusions"].append(
                    "OpenAI executor produced no final output"
                )
        return seal_trace(trace)
