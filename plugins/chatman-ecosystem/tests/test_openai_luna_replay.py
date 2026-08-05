from __future__ import annotations

import copy

import pytest
from openai_luna_protocol import OpenAIResponsesClient, RuntimeRefusal
from openai_luna_runtime import LunaHost, seal_trace, verify_trace
from openai_luna_testkit import (
    FakeStar,
    ScriptedResponses,
    profile,
    standard_script,
    standard_surfaces,
)


def make_trace() -> dict:
    scripted = ScriptedResponses(standard_script())
    return LunaHost(profile(), OpenAIResponsesClient(transport=scripted.create), FakeStar(), standard_surfaces()).run("execute", "ferroplan")


def test_identical_replay_has_identical_receipt() -> None:
    left = make_trace()
    right = make_trace()
    assert left == right
    assert left["trace_sha256"] == right["trace_sha256"]


def test_reseal_is_idempotent() -> None:
    trace = make_trace()
    assert seal_trace(trace) == trace


def test_every_material_mutation_falsifies_receipt() -> None:
    original = make_trace()
    mutations = [
        ("standing", "BLOCKED"),
        ("status", "refused"),
        ("model", "gpt-5.6"),
        ("final_output", "forged"),
    ]
    for key, value in mutations:
        changed = copy.deepcopy(original)
        changed[key] = value
        with pytest.raises(RuntimeRefusal) as caught:
            verify_trace(changed)
        assert caught.value.code == "TRACE_DIGEST_MISMATCH"
