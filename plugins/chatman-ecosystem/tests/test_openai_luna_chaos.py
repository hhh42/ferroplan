from __future__ import annotations

from pathlib import Path

import pytest
from mcp_client import McpClient, McpToolError
from openai_luna_protocol import OpenAIResponsesClient, RuntimeRefusal
from openai_luna_runtime import LunaHost
from openai_luna_testkit import (
    FakeStar,
    ScriptedResponses,
    fake_mcp_server,
    final_response,
    function_call,
    profile,
    standard_surfaces,
)


@pytest.mark.parametrize("mode", ["malformed", "crash"])
def test_mcp_process_faults_are_typed(tmp_path: Path, mode: str) -> None:
    launcher = fake_mcp_server(tmp_path / f"{mode}.py", "ferroplan", mode=mode)
    with pytest.raises(McpToolError):
        with McpClient(launcher=launcher, project_root=tmp_path, timeout=0.5) as client:
            client.list_tools()


def test_mcp_hang_times_out(tmp_path: Path) -> None:
    launcher = fake_mcp_server(tmp_path / "hang.py", "ferroplan", mode="hang")
    with pytest.raises(McpToolError, match="within"):
        with McpClient(launcher=launcher, project_root=tmp_path, timeout=0.05):
            pass


def test_openai_transport_fault_is_typed() -> None:
    def explode(_payload):
        raise OSError("network partition")

    with pytest.raises(RuntimeRefusal) as caught:
        LunaHost(profile(), OpenAIResponsesClient(transport=explode), FakeStar(), standard_surfaces()).run("execute", "ferroplan")
    assert caught.value.code == "OPENAI_TRANSPORT_ERROR"


def test_a2a_partition_does_not_grant_or_block_authority() -> None:
    class BrokenA2A:
        def probe(self):
            raise OSError("partition")

    scripted = ScriptedResponses([
        function_call("r1", ("f1", "ferroplan__solve", {}), ("o1", "ontostar__onto_admit_work_order", {})),
        final_response(),
    ])
    trace = LunaHost(profile(), OpenAIResponsesClient(transport=scripted.create), FakeStar(), standard_surfaces(), BrokenA2A()).run("execute", "ferroplan")
    assert trace["standing"] == "ALIVE"
    assert trace["a2a"]["authority"] == "coordination-only"
    assert "partition" in trace["a2a"]["error"]


def test_round_exhaustion_is_blocked_receipt() -> None:
    runtime = profile(max_rounds=2)
    scripted = ScriptedResponses([
        function_call("r1", ("f1", "ferroplan__solve", {})),
        function_call("r2", ("o1", "ontostar__onto_admit_work_order", {})),
    ])
    trace = LunaHost(runtime, OpenAIResponsesClient(transport=scripted.create), FakeStar(), standard_surfaces()).run("execute", "ferroplan")
    assert trace["standing"] == "BLOCKED"
    assert "Responses loop exceeded max_rounds" in trace["exclusions"]
