from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor

from openai_luna_protocol import TRACE_SCHEMA, OpenAIResponsesClient
from openai_luna_runtime import LunaHost, McpToolRegistry, seal_trace, verify_trace
from openai_luna_testkit import (
    FakeMcp,
    FakeStar,
    ScriptedResponses,
    profile,
    standard_script,
    standard_surfaces,
)


def test_discover_two_thousand_tools_within_bound() -> None:
    tools = [{"name": f"tool_{index}", "inputSchema": {"type": "object"}} for index in range(2000)]
    results = {tool["name"]: {"ok": True} for tool in tools}
    registry = McpToolRegistry({"bulk": FakeMcp(tools, results)}, max_tools=2000)
    discovered = registry.discover()
    assert len(discovered) == 2000
    assert len({tool["name"] for tool in discovered}) == 2000


def test_repeated_host_runs_do_not_leak_state() -> None:
    digests = []
    for _ in range(100):
        scripted = ScriptedResponses(standard_script())
        trace = LunaHost(profile(), OpenAIResponsesClient(transport=scripted.create), FakeStar(), standard_surfaces()).run("execute", "ferroplan")
        digests.append(trace["trace_sha256"])
    assert len(set(digests)) == 1


def test_parallel_trace_sealing_is_deterministic() -> None:
    payload = {"schema": TRACE_SCHEMA, "standing": "ALIVE", "items": list(range(1000))}
    with ThreadPoolExecutor(max_workers=16) as executor:
        traces = list(executor.map(lambda _: seal_trace(payload), range(256)))
    assert len({trace["trace_sha256"] for trace in traces}) == 1
    assert all(verify_trace(trace)["valid"] for trace in traces)
