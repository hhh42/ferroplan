from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from typing import Any

SCRIPTS = Path(__file__).resolve().parents[1] / "scripts"
sys.path.insert(0, str(SCRIPTS))

from mcp_client import McpClient  # noqa: E402
from openai_luna import (  # noqa: E402
    AUTHORIZED_EXECUTOR,
    LUNA_MODEL,
    MUSTAR_ENVELOPE_SCHEMA,
    MUSTAR_OBLIGATION_SCHEMA,
    PROFILE_SCHEMA,
    LunaHost,
    McpToolRegistry,
    OpenAIResponsesClient,
    RuntimeProfile,
    RuntimeRefusal,
    project_executor,
    validate_mustar_envelope,
)


def obligation(kind: str = "ImplementAcceptedDelta") -> dict[str, Any]:
    executor = "agent:claude-code" if kind == "ImplementAcceptedDelta" else "spec-kit"
    command = (
        "claude-code: implement accepted-delta"
        if kind == "ImplementAcceptedDelta"
        else "spec-kit: repair plan"
    )
    return {
        "schema": MUSTAR_OBLIGATION_SCHEMA,
        "id": "obl-ferroplan-123",
        "kind": kind,
        "target": "ferroplan",
        "station": "claude-code" if kind == "ImplementAcceptedDelta" else "spec-kit",
        "reason": "test",
        "preconditions": [],
        "blocks": [],
        "evidence_required": ["state_after_hash"],
        "dispatch": {"command": command, "executor": executor},
        "producer": "claude-code" if kind == "ImplementAcceptedDelta" else "spec-kit",
    }


def envelope(kind: str = "ImplementAcceptedDelta") -> dict[str, Any]:
    return {
        "schema": MUSTAR_ENVELOPE_SCHEMA,
        "command": "sr.mustar.next",
        "status": "pass",
        "target": "ferroplan",
        "line_status": "running",
        "work_unit": "unit-1",
        "data": {
            "obligation": obligation(kind),
            "obligation_hash": "blake3:abc123",
            "selected_by": "mustar",
        },
        "errors": [],
        "warnings": [],
        "next": None,
    }


class FakeMustar:
    def __init__(self, value: dict[str, Any]) -> None:
        self.value = value
        self.calls = 0

    def next(self, target: str) -> dict[str, Any]:
        self.calls += 1
        return self.value


class FakeMcp:
    def __init__(self, tools: list[dict[str, Any]], results: dict[str, Any]) -> None:
        self.tools = tools
        self.results = results
        self.calls: list[tuple[str, dict[str, Any]]] = []
        self.entered = False

    def __enter__(self):
        self.entered = True
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.entered = False

    def list_tools(self) -> list[dict[str, Any]]:
        return self.tools

    def call_tool(self, name: str, arguments: dict[str, Any]) -> dict[str, Any]:
        self.calls.append((name, arguments))
        return {"structuredContent": self.results[name]}


class ScriptedResponses:
    def __init__(self, responses: list[dict[str, Any]]) -> None:
        self.responses = list(responses)
        self.payloads: list[dict[str, Any]] = []

    def create(self, payload: dict[str, Any]) -> dict[str, Any]:
        self.payloads.append(payload)
        return self.responses.pop(0)


class LunaTests(unittest.TestCase):
    def profile(self) -> RuntimeProfile:
        return RuntimeProfile.from_dict(
            {
                "schema": PROFILE_SCHEMA,
                "model": LUNA_MODEL,
                "reasoning_effort": "medium",
                "max_rounds": 4,
                "required_servers": ["ferroplan", "ontostar"],
                "admission_tools": ["onto_validate"],
            }
        )

    def test_profile_refuses_unsuffixed_alias(self) -> None:
        with self.assertRaises(RuntimeRefusal) as caught:
            RuntimeProfile.from_dict(
                {
                    "schema": PROFILE_SCHEMA,
                    "model": "gpt-5.6",
                    "admission_tools": ["onto_validate"],
                }
            )
        self.assertEqual(caught.exception.code, "MODEL_NOT_LUNA")

    def test_executor_projection_preserves_authoritative_obligation(self) -> None:
        source = envelope()
        before = json.loads(json.dumps(source))
        projection = project_executor(validate_mustar_envelope(source))
        self.assertEqual(source, before)
        self.assertEqual(projection["original_executor"], "agent:claude-code")
        self.assertEqual(projection["effective_executor"], AUTHORIZED_EXECUTOR)
        self.assertTrue(projection["authorized"])
        self.assertEqual(projection["obligation_hash"], "blake3:abc123")

    def test_registry_namespaces_and_dispatches(self) -> None:
        client = FakeMcp(
            [{"name": "solve", "description": "Solve", "inputSchema": {"type": "object"}}],
            {"solve": {"plan": ["a"]}},
        )
        registry = McpToolRegistry({"ferroplan": client})
        tools = registry.discover()
        self.assertEqual(tools[0]["name"], "ferroplan__solve")
        result, identity = registry.call("ferroplan__solve", {"domain": "d"})
        self.assertEqual(result, {"plan": ["a"]})
        self.assertEqual(identity["server"], "ferroplan")
        self.assertEqual(client.calls, [("solve", {"domain": "d"})])

    def test_authorized_loop_requires_and_observes_ontostar(self) -> None:
        ferroplan = FakeMcp(
            [{"name": "solve", "description": "Solve", "inputSchema": {"type": "object"}}],
            {"solve": {"status": "solved", "plan": ["step"]}},
        )
        ontostar = FakeMcp(
            [
                {
                    "name": "onto_validate",
                    "description": "Validate",
                    "inputSchema": {"type": "object"},
                }
            ],
            {"onto_validate": {"status": "pass", "valid": True}},
        )
        scripted = ScriptedResponses(
            [
                {
                    "id": "resp-1",
                    "output": [
                        {
                            "type": "function_call",
                            "call_id": "call-1",
                            "name": "ferroplan__solve",
                            "arguments": "{\"domain\":\"d\"}",
                        },
                        {
                            "type": "function_call",
                            "call_id": "call-2",
                            "name": "ontostar__onto_validate",
                            "arguments": "{\"input\":\"x\",\"inline\":true}",
                        },
                    ],
                },
                {
                    "id": "resp-2",
                    "output": [
                        {
                            "type": "message",
                            "content": [{"type": "output_text", "text": "admitted"}],
                        }
                    ],
                },
            ]
        )
        host = LunaHost(
            self.profile(),
            OpenAIResponsesClient(transport=scripted.create),
            FakeMustar(envelope()),
            {"ferroplan": ferroplan, "ontostar": ontostar},
        )
        trace = host.run("execute", "ferroplan")
        self.assertEqual(trace["standing"], "ALIVE")
        self.assertEqual(trace["status"], "completed")
        self.assertTrue(trace["admission"]["positive"])
        self.assertEqual(trace["final_output"], "admitted")
        self.assertTrue(trace["trace_sha256"].startswith("sha256:"))
        second = scripted.payloads[1]
        self.assertEqual(second["previous_response_id"], "resp-1")
        self.assertEqual(len(second["input"]), 2)

    def test_model_output_without_ontostar_is_refused(self) -> None:
        ferroplan = FakeMcp(
            [{"name": "solve", "description": "Solve", "inputSchema": {"type": "object"}}],
            {"solve": {"status": "solved"}},
        )
        ontostar = FakeMcp(
            [{"name": "onto_validate", "description": "Validate", "inputSchema": {"type": "object"}}],
            {"onto_validate": {"status": "pass"}},
        )
        responses = ScriptedResponses(
            [
                {
                    "id": "resp-1",
                    "output": [
                        {
                            "type": "message",
                            "content": [{"type": "output_text", "text": "done"}],
                        }
                    ],
                }
            ]
        )
        host = LunaHost(
            self.profile(),
            OpenAIResponsesClient(transport=responses.create),
            FakeMustar(envelope()),
            {"ferroplan": ferroplan, "ontostar": ontostar},
        )
        trace = host.run("execute", "ferroplan")
        self.assertEqual(trace["standing"], "BLOCKED")
        self.assertIn("no positive OntoStar", trace["exclusions"][0])

    def test_non_implementation_obligation_never_calls_openai_or_mcp(self) -> None:
        responses = ScriptedResponses([])
        ferroplan = FakeMcp([], {})
        ontostar = FakeMcp([], {})
        host = LunaHost(
            self.profile(),
            OpenAIResponsesClient(transport=responses.create),
            FakeMustar(envelope("RepairPlan")),
            {"ferroplan": ferroplan, "ontostar": ontostar},
        )
        trace = host.run("execute", "ferroplan")
        self.assertEqual(trace["standing"], "BLOCKED")
        self.assertEqual(responses.payloads, [])
        self.assertFalse(ferroplan.entered)

    def test_mcp_client_tools_list_follows_cursor(self) -> None:
        client = McpClient(launcher=Path("unused"))
        writes: list[dict[str, Any]] = []
        replies = iter(
            [
                {"id": 1, "result": {"tools": [{"name": "one"}], "nextCursor": "next"}},
                {"id": 2, "result": {"tools": [{"name": "two"}]}},
            ]
        )

        def read_response(expected_id: int, timeout: float | None = None) -> dict[str, Any]:
            del expected_id, timeout
            return next(replies)

        client._write = writes.append  # type: ignore[method-assign]
        client._read_response = read_response  # type: ignore[method-assign]
        tools = client.list_tools()
        self.assertEqual([tool["name"] for tool in tools], ["one", "two"])
        self.assertNotIn("params", writes[0])
        self.assertEqual(writes[1]["params"], {"cursor": "next"})


if __name__ == "__main__":
    unittest.main()
