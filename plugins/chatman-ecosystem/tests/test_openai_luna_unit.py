from __future__ import annotations

import threading
import time
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest
from mcp_client import McpClient, McpToolError, tool_structured_result
from openai_luna_protocol import (
    LUNA_MODEL,
    PROFILE_SCHEMA,
    OpenAIResponsesClient,
    RuntimeProfile,
    RuntimeRefusal,
    canonical_json,
    sha256_digest,
    validate_star_envelope,
)
from openai_luna_runtime import LunaHost, McpToolRegistry, redact_for_trace
from openai_luna_testkit import (
    FakeMcp,
    FakeStar,
    ScriptedResponses,
    final_response,
    function_call,
    profile,
    standard_script,
    standard_surfaces,
    star_envelope,
)


def test_profile_accepts_explicit_bounds() -> None:
    value = profile()
    assert value.model == LUNA_MODEL
    assert value.max_tool_calls == 32
    assert value.max_discovered_tools == 128
    assert value.max_tool_result_bytes == 65536


@pytest.mark.parametrize(
    ("patch", "code"),
    [
        ({"schema": "wrong"}, "PROFILE_SCHEMA_MISMATCH"),
        ({"model": "gpt-5.6"}, "MODEL_NOT_LUNA"),
        ({"reasoning_effort": "turbo"}, "INVALID_REASONING_EFFORT"),
        ({"max_rounds": 0}, "INVALID_MAX_ROUNDS"),
        ({"max_tool_calls": True}, "INVALID_MAX_TOOL_CALLS"),
        ({"max_discovered_tools": 9000}, "INVALID_MAX_DISCOVERED_TOOLS"),
        ({"max_tool_result_bytes": 100}, "INVALID_MAX_TOOL_RESULT_BYTES"),
        ({"planning_tools": []}, "PLANNING_POLICY_EMPTY"),
        ({"admission_tools": []}, "ADMISSION_POLICY_EMPTY"),
        ({"required_servers": ["ferroplan", "ferroplan"]}, "REQUIRED_SERVERS_DUPLICATE"),
    ],
)
def test_profile_falsifiers(patch: dict[str, Any], code: str) -> None:
    value: dict[str, Any] = {
        "schema": PROFILE_SCHEMA,
        "model": LUNA_MODEL,
        "reasoning_effort": "medium",
        "max_rounds": 4,
        "required_servers": ["ferroplan", "ontostar"],
        "planning_tools": ["solve"],
        "admission_tools": ["onto_admit_work_order"],
        "star": {"mode": "mustar", "domain": "SYSTEM_DESIGN", "max_tasks": 4},
    }
    value.update(patch)
    with pytest.raises(RuntimeRefusal) as caught:
        RuntimeProfile.from_dict(value)
    assert caught.value.code == code


@pytest.mark.parametrize(
    ("patch", "code"),
    [
        ({"provisional": False}, "OSTAR_STAR_AUTHORITY_INVALID"),
        ({"authority": "executor"}, "OSTAR_STAR_AUTHORITY_INVALID"),
        ({"internal_actuation": True}, "OSTAR_STAR_ACTUATION_UNBOUNDED"),
        ({"results": []}, "OSTAR_STAR_RESULTS_EMPTY"),
        ({"star_classes": {}}, "OSTAR_STAR_CLASSES_INVALID"),
        ({"results": [{"title": ""}]}, "MUSTAR_RESULT_INVALID"),
    ],
)
def test_star_envelope_falsifiers(patch: dict[str, Any], code: str) -> None:
    value = star_envelope(**patch)
    with pytest.raises(RuntimeRefusal) as caught:
        validate_star_envelope(value)
    assert caught.value.code == code


def test_canonical_digest_is_order_independent() -> None:
    left = {"b": [2, 1], "a": {"z": True}}
    right = {"a": {"z": True}, "b": [2, 1]}
    assert canonical_json(left) == canonical_json(right)
    assert sha256_digest(left) == sha256_digest(right)


def test_registry_namespaces_and_resets() -> None:
    client = FakeMcp([{"name": "solve", "inputSchema": {"type": "object"}}], {"solve": {"ok": True}})
    registry = McpToolRegistry({"ferroplan": client})
    assert registry.discover()[0]["name"] == "ferroplan__solve"
    assert registry.discover()[0]["name"] == "ferroplan__solve"
    assert len(registry.mapping) == 1


def test_registry_collision_is_refused() -> None:
    client = FakeMcp(
        [
            {"name": "a b", "inputSchema": {"type": "object"}},
            {"name": "a@b", "inputSchema": {"type": "object"}},
        ],
        {"a b": {}, "a@b": {}},
    )
    with pytest.raises(RuntimeRefusal) as caught:
        McpToolRegistry({"server": client}).discover()
    assert caught.value.code == "MCP_TOOL_COLLISION"


def test_registry_result_bound_is_refused() -> None:
    client = FakeMcp([{"name": "big"}], {"big": {"payload": "x" * 2000}})
    registry = McpToolRegistry({"server": client}, max_result_bytes=100)
    registry.discover()
    with pytest.raises(RuntimeRefusal) as caught:
        registry.call("server__big", {})
    assert caught.value.code == "MCP_RESULT_BOUND_EXCEEDED"


def test_alive_requires_full_crown() -> None:
    responses = ScriptedResponses(standard_script())
    host = LunaHost(profile(), OpenAIResponsesClient(transport=responses.create), FakeStar(), standard_surfaces())
    trace = host.run("execute", "ferroplan")
    assert trace["standing"] == "ALIVE"
    assert trace["planning"]["positive"]
    assert trace["admission"]["positive"]
    assert trace["final_output"] == "done"
    assert trace["tool_calls"][0]["sequence"] == 1


def test_missing_final_text_blocks() -> None:
    responses = ScriptedResponses([
        function_call("r1", ("f1", "ferroplan__solve", {}), ("o1", "ontostar__onto_admit_work_order", {})),
        final_response(text=""),
    ])
    trace = LunaHost(profile(), OpenAIResponsesClient(transport=responses.create), FakeStar(), standard_surfaces()).run("execute", "ferroplan")
    assert trace["standing"] == "BLOCKED"
    assert "OpenAI executor produced no final output" in trace["exclusions"]


def test_duplicate_call_id_refused_and_clients_closed() -> None:
    surfaces = standard_surfaces()
    responses = ScriptedResponses([
        function_call("r1", ("same", "ferroplan__solve", {})),
        function_call("r2", ("same", "ontostar__onto_admit_work_order", {})),
    ])
    with pytest.raises(RuntimeRefusal) as caught:
        LunaHost(profile(), OpenAIResponsesClient(transport=responses.create), FakeStar(), surfaces).run("execute", "ferroplan")
    assert caught.value.code == "OPENAI_TOOL_CALL_ID_REUSED"
    assert surfaces["ferroplan"].exited == 1
    assert surfaces["ontostar"].exited == 1


def test_redaction_preserves_structure() -> None:
    value = {"api_key": "sk-secret", "nested": {"Authorization": "Bearer x", "value": 3}}
    assert redact_for_trace(value) == {"api_key": "***REDACTED***", "nested": {"Authorization": "***REDACTED***", "value": 3}}


def test_tool_structured_result_projections() -> None:
    assert tool_structured_result({"structuredContent": {"ok": True}}) == {"ok": True}
    assert tool_structured_result({"content": [{"type": "text", "text": '{"ok":true}'}]}) == {"ok": True}
    assert tool_structured_result({"content": [{"type": "text", "text": "plain"}]}) == "plain"



def test_mcp_timeout_is_total_deadline_not_per_message() -> None:
    client = McpClient(launcher=Path("unused"), timeout=0.03)
    client._process = SimpleNamespace(stderr=None)  # type: ignore[assignment]

    def noise() -> None:
        deadline = time.monotonic() + 0.15
        while time.monotonic() < deadline:
            client._line_queue.put('{"jsonrpc":"2.0","method":"noise"}\n')
            time.sleep(0.005)

    feeder = threading.Thread(target=noise, daemon=True)
    feeder.start()
    started = time.monotonic()
    with pytest.raises(McpToolError, match="within"):
        client._read_response(99)
    assert time.monotonic() - started < 0.10


def test_mcp_cursor_cycle_refused() -> None:
    client = McpClient(launcher=Path("unused"), max_tool_pages=3)
    writes: list[dict[str, Any]] = []
    replies = iter([
        {"id": 1, "result": {"tools": [], "nextCursor": "x"}},
        {"id": 2, "result": {"tools": [], "nextCursor": "x"}},
    ])
    client._write = writes.append  # type: ignore[method-assign]
    client._read_response = lambda expected_id, timeout=None: next(replies)  # type: ignore[method-assign]
    with pytest.raises(McpToolError, match="cursor cycle"):
        client.list_tools()
