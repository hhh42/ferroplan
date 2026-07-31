from __future__ import annotations

import json
from pathlib import Path

import pytest

from openai_luna_testkit import FakeStar, ScriptedResponses, profile, standard_script, standard_surfaces
from openai_luna_protocol import LUNA_MODEL, PROFILE_SCHEMA, TRACE_SCHEMA, OpenAIResponsesClient, RuntimeRefusal, load_profile
from openai_luna_runtime import LunaHost, verify_trace

PLUGIN = Path(__file__).resolve().parents[1]


def test_committed_profile_contract() -> None:
    committed = PLUGIN / "profiles" / "openai-luna.json"
    if not committed.exists():
        pytest.skip("sparse local verifier tree")
    loaded = load_profile(committed)
    assert loaded.model == LUNA_MODEL
    assert loaded.required_servers == ("ferroplan", "ontostar")


def test_trace_contract_and_digest() -> None:
    scripted = ScriptedResponses(standard_script())
    trace = LunaHost(profile(), OpenAIResponsesClient(transport=scripted.create), FakeStar(), standard_surfaces()).run("execute", "ferroplan")
    assert trace["schema"] == TRACE_SCHEMA
    assert trace["standing"] == "ALIVE"
    assert verify_trace(trace)["valid"] is True
    assert set(trace["verification_policy"]) == {"max_rounds", "max_tool_calls", "max_discovered_tools", "max_tool_result_bytes"}
    assert all("arguments_sha256" in call and "result_sha256" in call for call in trace["tool_calls"])


def test_trace_tamper_is_detected() -> None:
    scripted = ScriptedResponses(standard_script())
    trace = LunaHost(profile(), OpenAIResponsesClient(transport=scripted.create), FakeStar(), standard_surfaces()).run("execute", "ferroplan")
    trace["standing"] = "BLOCKED"
    with pytest.raises(RuntimeRefusal) as caught:
        verify_trace(trace)
    assert caught.value.code == "TRACE_DIGEST_MISMATCH"


def test_openai_payload_contract() -> None:
    scripted = ScriptedResponses(standard_script())
    LunaHost(profile(), OpenAIResponsesClient(transport=scripted.create), FakeStar(), standard_surfaces()).run("execute", "ferroplan")
    first = scripted.payloads[0]
    assert first["model"] == LUNA_MODEL
    assert first["tool_choice"] == "auto"
    names = {tool["name"] for tool in first["tools"]}
    assert names == {"ferroplan__solve", "ontostar__onto_admit_work_order"}
    task = json.loads(first["input"][0]["content"])
    assert task["star"]["authority"] == "proposer"
    assert task["star"]["internal_actuation"] is False
