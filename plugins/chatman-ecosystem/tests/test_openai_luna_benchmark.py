from __future__ import annotations

import statistics
import time

from openai_luna_testkit import FakeMcp
from openai_luna_runtime import McpToolRegistry, seal_trace


def _measure(callable_, rounds: int) -> list[float]:
    samples = []
    for _ in range(rounds):
        started = time.perf_counter()
        callable_()
        samples.append((time.perf_counter() - started) * 1000)
    return samples


def test_registry_discovery_benchmark() -> None:
    tools = [{"name": f"tool_{index}", "inputSchema": {"type": "object"}} for index in range(1000)]
    results = {tool["name"]: {"ok": True} for tool in tools}
    samples = _measure(lambda: McpToolRegistry({"bulk": FakeMcp(tools, results)}, max_tools=1000).discover(), 15)
    assert statistics.median(samples) < 250
    assert max(samples) < 1500


def test_trace_sealing_benchmark() -> None:
    trace = {
        "schema": "urn:chatman:a2a-openai-luna-trace:v2",
        "standing": "ALIVE",
        "tool_calls": [
            {"sequence": i, "arguments": {"i": i}, "result": {"ok": True}}
            for i in range(2000)
        ],
    }
    samples = _measure(lambda: seal_trace(trace), 20)
    assert statistics.median(samples) < 150
    assert max(samples) < 1000
