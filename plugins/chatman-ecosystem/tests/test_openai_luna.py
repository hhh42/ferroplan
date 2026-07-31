from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from typing import Any

SCRIPTS = Path(__file__).resolve().parents[1] / "scripts"
sys.path.insert(0, str(SCRIPTS))

from mcp_client import McpClient  # noqa: E402
from openai_luna_protocol import (  # noqa: E402
    LUNA_MODEL,
    OSTAR_STAR_SCHEMA,
    PROFILE_SCHEMA,
    OpenAIResponsesClient,
    RuntimeProfile,
    RuntimeRefusal,
    validate_star_envelope,
)
from openai_luna_runtime import LunaHost, McpToolRegistry  # noqa: E402


def mustar_result() -> dict[str, Any]:
    return {
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


def star_envelope() -> dict[str, Any]:
    return {
        "schema": OSTAR_STAR_SCHEMA,
        "mode": "mustar",
        "target": "ferroplan",
        "provisional": True,
        "authority": "proposer",
        "internal_actuation": False,
        "star_classes": {
            "planner": "MuStarPlanner",
            "executor": "MuStarExecutor",
        },
        "results": [mustar_result()],
    }


class FakeStar:
    def __init__(self, value: dict[str, Any]) -> None:
        self.value = value
        self.calls: list[tuple[str, str, RuntimeProfile]] = []

    def solve(
        self,
        prompt: str,
        target: str,
        profile: RuntimeProfile,
    ) -> dict[str, Any]:
        self.calls.append((prompt, target, profile))
        return self.value


class FakeMcp:
    def __init__(self, tools: list[dict[str, Any]], results: dict[str, Any]) -> None:
        self.tools = tools
        self.results = results
        self.calls: list[tuple[str, dict[str, Any]]] = []

    def __enter__(self):
        return self

    def __exit__(self, *exc_info: object) -> None:
        return None

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


class StarRuntimeTests(unittest.TestCase):
    def profile(self) -> RuntimeProfile:
        return RuntimeProfile.from_dict(
            {
                "schema": PROFILE_SCHEMA,
                "model": LUNA_MODEL,
                "reasoning_effort": "medium",
                "max_rounds": 4,
                "required_servers": ["ferroplan", "ontostar"],
                "planning_tools": ["solve"],
                "admission_tools": ["onto_admit_work_order"],
                "star": {
                    "mode": "mustar",
                    "domain": "SYSTEM_DESIGN",
                    "max_tasks": 4,
                },
            }
        )

    def test_profile_refuses_old_schema(self) -> None:
        with self.assertRaises(RuntimeRefusal):
            RuntimeProfile.from_dict(
                {
                    "schema": "urn:chatman:openai-luna-profile:v1",
                    "model": LUNA_MODEL,
                }
            )

    def test_star_envelope_requires_provisional_no_actuation(self) -> None:
        value = star_envelope()
        self.assertEqual(validate_star_envelope(value), value)
        value["internal_actuation"] = True
        with self.assertRaises(RuntimeRefusal) as caught:
            validate_star_envelope(value)
        self.assertEqual(
            caught.exception.code,
            "OSTAR_STAR_ACTUATION_UNBOUNDED",
        )

    def test_registry_namespaces_tools(self) -> None:
        client = FakeMcp(
            [{"name": "solve", "inputSchema": {"type": "object"}}],
            {"solve": {"plan": ["a"]}},
        )
        registry = McpToolRegistry({"ferroplan": client})
        tools = registry.discover()
        self.assertEqual(tools[0]["name"], "ferroplan__solve")

    def test_alive_requires_star_ferroplan_ontostar_and_final_text(self) -> None:
        ferroplan = FakeMcp(
            [
                {
                    "name": "solve",
                    "description": "Solve",
                    "inputSchema": {"type": "object"},
                }
            ],
            {"solve": {"status": "solved", "plan": ["step"]}},
        )
        ontostar = FakeMcp(
            [
                {
                    "name": "onto_admit_work_order",
                    "description": "Admit",
                    "inputSchema": {"type": "object"},
                }
            ],
            {
                "onto_admit_work_order": {
                    "admission": "admitted",
                    "receipt_hash": "abc",
                }
            },
        )
        responses = ScriptedResponses(
            [
                {
                    "id": "resp-1",
                    "output": [
                        {
                            "type": "function_call",
                            "call_id": "f1",
                            "name": "ferroplan__solve",
                            "arguments": "{}",
                        },
                        {
                            "type": "function_call",
                            "call_id": "o1",
                            "name": "ontostar__onto_admit_work_order",
                            "arguments": "{}",
                        },
                    ],
                },
                {
                    "id": "resp-2",
                    "output": [
                        {
                            "type": "message",
                            "content": [
                                {"type": "output_text", "text": "done"}
                            ],
                        }
                    ],
                },
            ]
        )
        star = FakeStar(star_envelope())
        host = LunaHost(
            self.profile(),
            OpenAIResponsesClient(transport=responses.create),
            star,
            {"ferroplan": ferroplan, "ontostar": ontostar},
        )
        trace = host.run("execute", "ferroplan")
        self.assertEqual(trace["standing"], "ALIVE")
        self.assertTrue(trace["planning"]["positive"])
        self.assertTrue(trace["admission"]["positive"])
        self.assertEqual(star.calls[0][0], "execute")
        first_payload = responses.payloads[0]
        task = json.loads(first_payload["input"][0]["content"])
        self.assertTrue(task["star"]["provisional"])

    def test_ontostar_without_ferroplan_is_blocked(self) -> None:
        ferroplan = FakeMcp(
            [{"name": "solve", "inputSchema": {"type": "object"}}],
            {"solve": {"status": "solved"}},
        )
        ontostar = FakeMcp(
            [
                {
                    "name": "onto_admit_work_order",
                    "inputSchema": {"type": "object"},
                }
            ],
            {"onto_admit_work_order": {"admission": "admitted"}},
        )
        responses = ScriptedResponses(
            [
                {
                    "id": "resp-1",
                    "output": [
                        {
                            "type": "function_call",
                            "call_id": "o1",
                            "name": "ontostar__onto_admit_work_order",
                            "arguments": "{}",
                        }
                    ],
                },
                {
                    "id": "resp-2",
                    "output": [
                        {
                            "type": "message",
                            "content": [
                                {"type": "output_text", "text": "done"}
                            ],
                        }
                    ],
                },
            ]
        )
        host = LunaHost(
            self.profile(),
            OpenAIResponsesClient(transport=responses.create),
            FakeStar(star_envelope()),
            {"ferroplan": ferroplan, "ontostar": ontostar},
        )
        trace = host.run("execute", "ferroplan")
        self.assertEqual(trace["standing"], "BLOCKED")
        self.assertIn(
            "no positive Ferroplan planning witness was observed",
            trace["exclusions"],
        )

    def test_ferroplan_without_ontostar_is_blocked(self) -> None:
        ferroplan = FakeMcp(
            [{"name": "solve", "inputSchema": {"type": "object"}}],
            {"solve": {"status": "solved"}},
        )
        ontostar = FakeMcp(
            [
                {
                    "name": "onto_admit_work_order",
                    "inputSchema": {"type": "object"},
                }
            ],
            {"onto_admit_work_order": {"admission": "admitted"}},
        )
        responses = ScriptedResponses(
            [
                {
                    "id": "resp-1",
                    "output": [
                        {
                            "type": "function_call",
                            "call_id": "f1",
                            "name": "ferroplan__solve",
                            "arguments": "{}",
                        }
                    ],
                },
                {
                    "id": "resp-2",
                    "output": [
                        {
                            "type": "message",
                            "content": [
                                {"type": "output_text", "text": "done"}
                            ],
                        }
                    ],
                },
            ]
        )
        host = LunaHost(
            self.profile(),
            OpenAIResponsesClient(transport=responses.create),
            FakeStar(star_envelope()),
            {"ferroplan": ferroplan, "ontostar": ontostar},
        )
        trace = host.run("execute", "ferroplan")
        self.assertEqual(trace["standing"], "BLOCKED")
        self.assertIn(
            "no positive OntoStar admission witness was observed",
            trace["exclusions"],
        )

    def test_mcp_client_tools_list_follows_cursor(self) -> None:
        client = McpClient(launcher=Path("unused"))
        writes: list[dict[str, Any]] = []
        replies = iter(
            [
                {
                    "id": 1,
                    "result": {
                        "tools": [{"name": "one"}],
                        "nextCursor": "next",
                    },
                },
                {"id": 2, "result": {"tools": [{"name": "two"}]}},
            ]
        )

        def read_response(
            expected_id: int,
            timeout: float | None = None,
        ) -> dict[str, Any]:
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
